use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::future::{join_all, JoinAll};
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

fn block_on(c: &mut Criterion) {
    c.bench_function("tio on 0 yield", |b| {
        b.iter(|| tio::task::block_on(black_box(Yields(0))))
    });
    c.bench_function("tio on 10 yield", |b| {
        b.iter(|| tio::task::block_on(black_box(Yields(10))))
    });
    c.bench_function("tio on 50 yield", |b| {
        b.iter(|| tio::task::block_on(black_box(Yields(50))))
    });
    c.bench_function("async-std on 0 yield", |b| {
        b.iter(|| async_std::task::block_on(black_box(Yields(0))))
    });
    c.bench_function("async-std on 10 yield", |b| {
        b.iter(|| async_std::task::block_on(black_box(Yields(10))))
    });
    c.bench_function("async-std on 50 yield", |b| {
        b.iter(|| async_std::task::block_on(black_box(Yields(50))))
    });
}

fn spawn(c: &mut Criterion) {
    const TASKS: usize = 200;
    fn tio_tasks(yields: usize) -> JoinAll<tio::task::JoinHandle<()>> {
        join_all((0..TASKS).map(|_| tio::task::spawn(black_box(Yields(yields)))))
    }

    fn async_std_tasks(yields: usize) -> JoinAll<async_std::task::JoinHandle<()>> {
        join_all((0..TASKS).map(|_| async_std::task::spawn(black_box(Yields(yields)))))
    }

    c.bench_function("tio on 0 yield", |b| {
        b.iter(|| async_std::task::block_on(tio_tasks(0)))
    });
    c.bench_function("tio on 10 yield", |b| {
        b.iter(|| async_std::task::block_on(tio_tasks(10)))
    });
    c.bench_function("tio on 50 yield", |b| {
        b.iter(|| async_std::task::block_on(tio_tasks(50)))
    });
    c.bench_function("async-std on 0 yield", |b| {
        b.iter(|| async_std::task::block_on(async_std_tasks(0)))
    });
    c.bench_function("async-std on 10 yield", |b| {
        b.iter(|| async_std::task::block_on(async_std_tasks(10)))
    });
    c.bench_function("async-std on 50 yield", |b| {
        b.iter(|| async_std::task::block_on(async_std_tasks(50)))
    });
}

criterion_group!(benches, block_on, spawn);
criterion_main!(benches);
