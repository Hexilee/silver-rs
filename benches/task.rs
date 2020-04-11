use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::executor;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tio::task;

struct Yields(u32);

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
        b.iter(|| task::block_on(black_box(Yields(0))))
    });
    c.bench_function("tio on 10 yield", |b| {
        b.iter(|| task::block_on(black_box(Yields(10))))
    });
    c.bench_function("tio on 50 yield", |b| {
        b.iter(|| task::block_on(black_box(Yields(50))))
    });
    c.bench_function("futures on 0 yield", |b| {
        b.iter(|| executor::block_on(black_box(Yields(0))))
    });
    c.bench_function("futures on 10 yield", |b| {
        b.iter(|| executor::block_on(black_box(Yields(10))))
    });
    c.bench_function("futures on 50 yield", |b| {
        b.iter(|| executor::block_on(black_box(Yields(50))))
    });
}

criterion_group!(benches, block_on);
criterion_main!(benches);
