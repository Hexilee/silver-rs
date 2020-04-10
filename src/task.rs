use async_task::waker_fn;
use crossbeam_utils::sync::Parker;
use pin_utils::pin_mut;
use std::future::Future;
use std::task::{Context, Poll, Waker};

pub fn block_on<F>(fut: F) -> F::Output
where
    F: Future,
{
    thread_local! {
        static SCHEDULE: (Parker, Waker) = {
            let parker = Parker::new();
            let unparker = parker.unparker().clone();
            let waker = waker_fn(move || unparker.unpark());
            (parker, waker)
        };
    }
    pin_mut!(fut);

    SCHEDULE.with(|(parker, waker): &(Parker, Waker)| {
        let mut ctx = Context::from_waker(waker);
        loop {
            match fut.as_mut().poll(&mut ctx) {
                Poll::Pending => parker.park(),
                Poll::Ready(output) => break output,
            }
        }
    })
}
