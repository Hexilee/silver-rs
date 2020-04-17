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
