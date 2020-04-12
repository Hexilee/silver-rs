mod block;
mod blocking;
mod spawn;

pub use block::block_on;
pub use blocking::spawn_blocking;
pub use spawn::spawn;

use futures::task::{Context, Poll};
use std::future::Future;
use std::panic::resume_unwind;
use std::pin::Pin;
use std::thread::Result;

type Task = async_task::Task<()>;

pub struct JoinHandle<T>(async_task::JoinHandle<Result<T>, ()>);

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0)
            .poll(cx)
            .map(|opt| match opt.expect("task cannot be canceled") {
                Ok(ret) => ret,
                Err(err) => resume_unwind(err),
            })
    }
}
