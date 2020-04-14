use super::{JoinHandle, Task};
use crossbeam_deque::{Injector, Stealer, Worker};
use crossbeam_queue::ArrayQueue;
use crossbeam_utils::sync::{Parker, Unparker};
use futures::FutureExt;
use once_cell::sync::Lazy;
use std::future::Future;
use std::iter;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::thread;

struct Pool {
    injector: Injector<Task>,
    unparkers: ArrayQueue<Unparker>,
}

impl Pool {
    fn new(cpus: usize) -> Self {
        Self {
            injector: Injector::new(),
            unparkers: ArrayQueue::new(cpus),
        }
    }
}

static POOL: Lazy<Arc<Pool>> = Lazy::new(|| {
    let nums = num_cpus::get();
    let pool = Arc::new(Pool::new(nums));
    let workers = (0..nums).map(|_| Worker::new_fifo()).collect::<Vec<_>>();
    let stealers = workers
        .iter()
        .map(|worker| worker.stealer())
        .collect::<Vec<Stealer<Task>>>();
    for (index, worker) in workers.into_iter().enumerate() {
        let stealers = stealers.clone();
        let pool = pool.clone();
        thread::Builder::new()
            .name(format!("tio/async{}", index))
            .spawn(move || {
                let parker = Parker::new();
                loop {
                    if let Some(task) = find_task(&worker, &pool.injector, &stealers) {
                        task.run();
                    } else {
                        pool.unparkers
                            .push(parker.unparker().clone())
                            .expect("fail to push unparker");
                        parker.park()
                    }
                }
            })
            .expect("fail to start thread");
    }
    pool
});

#[inline]
fn find_task<T>(local: &Worker<T>, global: &Injector<T>, stealers: &[Stealer<T>]) -> Option<T> {
    // Pop a task from the local queue, if not empty.
    local.pop().or_else(|| {
        // Otherwise, we need to look for a task elsewhere.
        iter::repeat_with(|| {
            // Try stealing a batch of tasks from the global queue.
            global
                .steal_batch_and_pop(local)
                // Or try stealing a task from one of the other threads.
                .or_else(|| stealers.iter().map(|s| s.steal()).collect())
        })
        // Loop while no task was stolen and any steal operation needs to be retried.
        .find(|s| !s.is_retry())
        // Extract the stolen task, if there is one.
        .and_then(|s| s.success())
    })
}

#[inline]
fn schedule(t: Task) {
    POOL.injector.push(t);
    if let Ok(unparker) = POOL.unparkers.pop() {
        unparker.unpark()
    }
}

pub fn spawn<F, R>(fut: F) -> JoinHandle<R>
where
    R: 'static + Send,
    F: 'static + Send + Future<Output = R>,
{
    let fut = AssertUnwindSafe(fut).catch_unwind();
    let (task, handler) = async_task::spawn(fut, schedule, ());
    task.schedule();
    JoinHandle(handler)
}

#[cfg(test)]
mod tests {
    use super::spawn;
    use crate::task::block_on;

    #[test]
    fn basic() {
        assert_eq!(1, block_on(spawn(async { 1 })));
    }
}
