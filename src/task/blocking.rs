use super::{JoinHandle, Task};
use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

static FREE_THREADS: AtomicUsize = AtomicUsize::new(0);
const TIMEOUT: Duration = Duration::from_secs(1);

static POOL: Lazy<Sender<Task>> = Lazy::new(|| {
    let (sender, recv) = unbounded();
    start_thread(recv);
    sender
});

fn start_thread(recv: Receiver<Task>) {
    thread::Builder::new()
        .name("tio/blocking".to_string())
        .spawn(move || {
            FREE_THREADS.fetch_add(1, Ordering::SeqCst);
            loop {
                let result = recv.recv_timeout(TIMEOUT);
                let mut task = match result {
                    Ok(task) => task,
                    Err(_) => {
                        if FREE_THREADS.fetch_sub(1, Ordering::SeqCst) == 1 {
                            FREE_THREADS.fetch_add(1, Ordering::SeqCst);
                            continue;
                        }
                        // stop thread
                        break;
                    }
                };

                if FREE_THREADS.fetch_sub(1, Ordering::SeqCst) == 1 {
                    start_thread(recv.clone())
                }

                loop {
                    task.run();
                    task = match recv.try_recv() {
                        Ok(t) => t,
                        Err(_) => break,
                    }
                }

                if FREE_THREADS.load(Ordering::SeqCst) > 0 {
                    break;
                }

                FREE_THREADS.fetch_add(1, Ordering::SeqCst);
            }
        })
        .expect("cannot start a blocking thread");
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`task::block_on`], [`task::spawn`].
///
/// [`task::block_on`]: fn.block_on.html
/// [`task::spawn`]: fn.spawn.html
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
/// let val = task::spawn_blocking(|| {
///     println!("long-running task here");
///     1
/// }).await;
/// assert_eq!(1, val);
/// #
/// # })
/// ```
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    R: 'static + Send,
    F: 'static + Send + FnOnce() -> R,
{
    let (task, handler) = async_task::spawn(
        async move { catch_unwind(AssertUnwindSafe(f)) },
        |t| POOL.send(t).expect("No blocking thread started"),
        (),
    );
    task.schedule();
    JoinHandle(handler)
}
