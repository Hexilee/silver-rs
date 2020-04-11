use async_task::waker_fn;
use crossbeam_utils::sync::Parker;
use futures::pin_mut;
use std::cell::Cell;
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

        static IS_RUNNING: Cell<bool> = Cell::new(false);
    }

    struct RunningGuard;

    impl RunningGuard {
        fn new() -> Self {
            IS_RUNNING.with(|is_running| {
                if is_running.get() {
                    panic!("recursively call `block_on`")
                }
                is_running.set(true)
            });
            Self
        }
    }

    impl Drop for RunningGuard {
        fn drop(&mut self) {
            IS_RUNNING.with(|is_running| is_running.set(false));
        }
    }

    pin_mut!(fut);
    let _guard = RunningGuard::new();
    SCHEDULE.with(|(parker, waker)| {
        let mut ctx = Context::from_waker(waker);
        loop {
            match fut.as_mut().poll(&mut ctx) {
                Poll::Pending => parker.park(),
                Poll::Ready(output) => break output,
            }
        }
    })
}

#[cfg(any(test, bench))]
mod tests {
    use super::block_on;

    #[test]
    #[should_panic]
    fn recursive_block_on() {
        block_on(async {
            block_on(async {});
        });
    }
}
