mod block;
mod blocking;
mod spawn;

pub use block::block_on;
pub use spawn::spawn;

use futures::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;

type Task = async_task::Task<()>;

pub struct JoinHandler<T>(async_task::JoinHandle<T, ()>);

impl<T> Future for JoinHandler<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(opt) => Poll::Ready(opt.unwrap()),
        }
    }
}
