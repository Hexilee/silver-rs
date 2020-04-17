//! Types and traits for working with asynchronous tasks.
//!
//! This module is similar to [`std::thread`], except it uses asynchronous tasks in place of
//! threads.
//!
//! [`std::thread`]: https://doc.rust-lang.org/std/thread
//!
//! ## The task model
//!
//! An executing asynchronous Rust program consists of a collection of native OS threads, on top of
//! which multiple stackless coroutines are multiplexed. We refer to these as "tasks".  Tasks can
//! be named, and provide some built-in support for synchronization.
//!
//! Fatal logic errors in Rust cause *thread panic*, during which a thread will unwind the stack,
//! running destructors and freeing owned resources. If a panic occurs inside a task, there is no
//! meaningful way of recovering, so the panic will propagate through task which is awaiting [`JoinHandle`].
//!
//! ## Spawning a task
//!
//! A new task can be spawned using the [`task::spawn`][`spawn`] function:
//!
//! ```no_run
//! use tio::task;
//!
//! task::spawn(async {
//!     // some work here
//! });
//! ```
//!
//! In this example, the spawned task is "detached" from the current task. This means that it can
//! outlive its parent (the task that spawned it), unless this parent is the root task.
//!
//! The root task can also wait on the completion of the child task; a call to [`spawn`] produces a
//! [`JoinHandle`], which implements `Future` and can be `await`ed:
//!
//! ```
//! use tio::task;
//!
//! # tio::task::block_on(async {
//! #
//! let child = task::spawn(async {
//!     // some work here
//! });
//! // some work here
//! let res = child.await;
//! #
//! # })
//! ```
//!
//! The `await` operator returns the final value produced by the child task.
//!
//! [`spawn`]: fn.spawn.html
//! [`JoinHandle`]: struct.JoinHandle.html
//! [`panic!`]: https://doc.rust-lang.org/std/macro.panic.html

mod block;
mod blocking;
mod yield_now;

#[cfg(feature = "async-rt")]
mod spawn;

#[cfg(feature = "async-rt")]
pub use spawn::spawn;

#[cfg(feature = "timer")]
mod timer;

#[cfg(feature = "timer")]
pub use timer::{interval, sleep, Interval};

pub use block::block_on;
pub use blocking::spawn_blocking;
pub use yield_now::yield_now;

use std::future::Future;
use std::panic::resume_unwind;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::Result;

type Task = async_task::Task<()>;

/// A handle that awaits the result of a task.
///
/// Dropping a [`JoinHandle`] will detach the task, meaning that there is no longer
/// a handle to the task and no way to `join` on it.
///
/// Created when a task is [spawned].
///
/// [spawned]: fn.spawn.html
pub struct JoinHandle<T>(async_task::JoinHandle<Result<T>, ()>);

impl<T> Future for JoinHandle<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|opt| {
            match opt.expect("task should not be canceled") {
                Ok(ret) => ret,
                Err(err) => resume_unwind(err),
            }
        })
    }
}
