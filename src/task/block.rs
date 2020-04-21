use async_task::waker_fn;
use crossbeam_utils::sync::Parker;
use futures::pin_mut;
use std::cell::Cell;
use std::future::Future;
use std::task::{Context, Poll, Waker};

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
    #[inline]
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
    #[inline]
    fn drop(&mut self) {
        IS_RUNNING.with(|is_running| is_running.set(false));
    }
}

/// Spawns a task and blocks the current thread on its result.
///
/// Calling this function is similar to [spawning] a thread and immediately [joining] it, except an
/// asynchronous task will be spawned.
///
/// See also: [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// [spawning]: https://doc.rust-lang.org/std/thread/fn.spawn.html
/// [joining]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
///
/// # Examples
///
/// ```no_run
/// use tio::task;
/// let val = task::block_on(async {
///     println!("Hello, world!");
///     1
/// });
/// assert_eq!(1, val);
/// ```
pub fn block_on<F>(fut: F) -> F::Output
where
    F: Future,
{
    let _guard = RunningGuard::new();
    pin_mut!(fut);
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

#[cfg(test)]
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
