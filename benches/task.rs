#![feature(test)]
extern crate test;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct Yields(usize);

impl Future for Yields {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 {
            Poll::Ready(())
        } else {
            self.0 -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

mod block_on {
    use super::Yields;
    use test::Bencher;

    #[bench]
    fn tio_block_on_0_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(0)))
    }
    #[bench]
    fn tio_block_on_10_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(10)))
    }
    #[bench]
    fn tio_block_on_50_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(50)))
    }
    #[bench]
    fn async_std_block_on_0_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(0)))
    }
    #[bench]
    fn async_std_block_on_10_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(10)))
    }
    #[bench]
    fn async_std_block_on_50_yield(b: &mut Bencher) {
        b.iter(|| tio::task::block_on(Yields(50)))
    }
}

mod spawn {
    use super::Yields;
    use async_std::task::block_on;
    use futures::future::{join_all, JoinAll};
    use test::Bencher;

    const TASKS: usize = 200;
    fn tio_tasks(yields: usize) -> JoinAll<tio::task::JoinHandle<()>> {
        join_all((0..TASKS).map(|_| tio::task::spawn(Yields(yields))))
    }

    fn async_std_tasks(yields: usize) -> JoinAll<async_std::task::JoinHandle<()>> {
        join_all((0..TASKS).map(|_| async_std::task::spawn(Yields(yields))))
    }

    #[bench]
    fn tio_spawn_0_yield(b: &mut Bencher) {
        b.iter(|| block_on(tio_tasks(0)))
    }
    #[bench]
    fn tio_spawn_10_yield(b: &mut Bencher) {
        b.iter(|| block_on(tio_tasks(10)))
    }
    #[bench]
    fn tio_spawn_50_yield(b: &mut Bencher) {
        b.iter(|| block_on(tio_tasks(50)))
    }

    #[bench]
    fn async_std_spawn_0_yield(b: &mut Bencher) {
        b.iter(|| block_on(async_std_tasks(0)))
    }
    #[bench]
    fn async_std_spawn_10_yield(b: &mut Bencher) {
        b.iter(|| block_on(async_std_tasks(10)))
    }
    #[bench]
    fn async_std_spawn_50_yield(b: &mut Bencher) {
        b.iter(|| block_on(async_std_tasks(50)))
    }
}

mod spawn_blocking {
    use async_std::task::block_on;
    use futures::future::join_all;
    use std::fs::read;
    use test::Bencher;

    const TASKS: usize = 200;
    const FILE_PATH: &str = "Cargo.toml";

    #[bench]
    fn tio_spawn_blocking(b: &mut Bencher) {
        let tasks =
            || join_all((0..TASKS).map(|_| tio::task::spawn_blocking(|| read(FILE_PATH).unwrap())));
        b.iter(|| block_on(tasks()))
    }

    #[bench]
    fn async_std_spawn_blocking(b: &mut Bencher) {
        let tasks = || {
            join_all(
                (0..TASKS).map(|_| async_std::task::spawn_blocking(|| read(FILE_PATH).unwrap())),
            )
        };
        b.iter(|| block_on(tasks()))
    }
}
