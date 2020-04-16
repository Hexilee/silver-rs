use futures::future::poll_fn;
use std::task::Poll;

/// Cooperatively gives up a timeslice to the task scheduler.
///
/// Calling this function will move the currently executing future to the back
/// of the execution queue, making room for other futures to execute. This is
/// especially useful after running CPU-intensive operations inside a future.
///
/// See also [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # tio::task::block_on(async {
/// #
/// use tio::task;
///
/// task::yield_now().await;
/// #
/// # })
/// ```
pub async fn yield_now() {
    let mut has_yielded = false;
    poll_fn(|cx| {
        if has_yielded {
            Poll::Ready(())
        } else {
            has_yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}
