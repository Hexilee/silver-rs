use super::{JoinHandle, Task};
use crossbeam_deque::{Injector, Stealer, Worker};
use futures::FutureExt;
use once_cell::sync::Lazy;
use std::future::Future;
use std::iter;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

const TIMEOUT: Duration = Duration::from_millis(10);

static POOL: Lazy<Arc<Injector<Task>>> = Lazy::new(|| {
    let injector = Arc::new(Injector::new());
    let nums = num_cpus::get();
    let workers = (0..nums).map(|_| Worker::new_fifo()).collect::<Vec<_>>();
    let stealers = workers
        .iter()
        .map(|worker| worker.stealer())
        .collect::<Vec<Stealer<Task>>>();
    for (index, worker) in workers.into_iter().enumerate() {
        let injector = injector.clone();
        let stealers = stealers.clone();
        thread::Builder::new()
            .name(format!("tio/async{}", index))
            .spawn(move || loop {
                if let Some(task) = find_task(&worker, &injector, &stealers) {
                    task.run()
                } else {
                    thread::sleep(TIMEOUT)
                }
            })
            .expect("fail to start thread");
    }
    injector
});

pub fn spawn<F, R>(fut: F) -> JoinHandle<R>
where
    R: 'static + Send,
    F: 'static + Send + Future<Output = R>,
{
    let (task, handler) =
        async_task::spawn(AssertUnwindSafe(fut).catch_unwind(), |f| POOL.push(f), ());
    task.schedule();
    JoinHandle(handler)
}
