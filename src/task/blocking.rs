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

enum GuardType {
    Free,
    Busy,
}

struct ThreadGuard {
    free_threads: usize,
    typ: GuardType,
}

use GuardType::*;

impl ThreadGuard {
    fn busy() -> Self {
        Self {
            free_threads: FREE_THREADS.fetch_sub(1, Ordering::SeqCst),
            typ: Busy,
        }
    }

    fn free() -> Self {
        Self {
            free_threads: FREE_THREADS.fetch_add(1, Ordering::SeqCst),
            typ: Free,
        }
    }
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        match self.typ {
            Busy => FREE_THREADS.fetch_add(1, Ordering::SeqCst),
            Free => FREE_THREADS.fetch_sub(1, Ordering::SeqCst),
        };
    }
}

fn start_thread(recv: Receiver<Task>) {
    let _ = ThreadGuard::free();
    thread::Builder::new()
        .name("tio/blocking".to_string())
        .spawn(move || loop {
            let result = recv.recv_timeout(TIMEOUT);
            let guard = ThreadGuard::busy();
            let mut task = match result {
                Ok(task) => task,
                Err(_) => {
                    if guard.free_threads == 1 {
                        continue;
                    }
                    // stop thread
                    return;
                }
            };

            if guard.free_threads == 1 {
                start_thread(recv.clone())
            }

            loop {
                task.run();
                task = match recv.try_recv() {
                    Ok(t) => t,
                    Err(_) => break,
                }
            }
        })
        .expect("cannot start a blocking thread");
}

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
