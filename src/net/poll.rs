use crossbeam_queue::SegQueue;
use mio::event;
use mio::{Events, Interest, Poll, Registry, Token};
use once_cell::sync::Lazy;
use slab::Slab;
use std::io;
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Waker;
use std::thread;

const EVENTS: usize = 1 << 12;
const THREAD_NAME: &str = "tio/poll";
const ALL_INTEREST: Interest = Interest::READABLE.add(Interest::WRITABLE);

static REACTOR: Lazy<Reactor> = Lazy::new(|| {
    let mut poll = match Poll::new() {
        Ok(p) => p,
        Err(err) => panic!("fail to construct a mio::Poll: {}", err),
    };
    let registry = match poll.registry().try_clone() {
        Ok(r) => r,
        Err(err) => panic!("fail to clone mio::Registry: {}", err),
    };
    let wakers = Arc::new(RwLock::new(Slab::<Wakers>::new()));
    let wakers_cloned = wakers.clone();
    thread::Builder::new()
        .name(THREAD_NAME.to_string())
        .spawn(move || {
            let mut events = Events::with_capacity(EVENTS);
            loop {
                if let Err(err) = poll.poll(&mut events, None) {
                    panic!("poll error: {}", err)
                }
                let wakers = wakers_cloned.read().expect("entry lock poisoned");
                for event in events.iter() {
                    let token = event.token();
                    if event.is_readable() {
                        while let Ok(waker) = wakers[token.0].reader.pop() {
                            waker.wake()
                        }
                    }
                    if event.is_writable() {
                        while let Ok(waker) = wakers[token.0].writer.pop() {
                            waker.wake()
                        }
                    }
                }
            }
        })
        .expect(&format!("fail to spawn thread {}", THREAD_NAME));
    Reactor { registry, wakers }
});

struct Reactor {
    registry: Registry,
    wakers: Arc<RwLock<Slab<Wakers>>>,
}

pub struct Watcher<S>
where
    S: event::Source,
{
    pub(crate) index: usize,
    pub(crate) source: S,
}

struct Wakers {
    reader: SegQueue<Waker>,
    writer: SegQueue<Waker>,
}

impl Wakers {
    fn new() -> Self {
        Wakers {
            reader: SegQueue::new(),
            writer: SegQueue::new(),
        }
    }
}

impl<S> Watcher<S>
where
    S: event::Source,
{
    fn new(mut source: S) -> io::Result<Self> {
        let index = REACTOR
            .wakers
            .write()
            .expect("entry lock poisoned")
            .insert(Wakers::new());
        REACTOR
            .registry
            .register(&mut source, Token(index), ALL_INTEREST)?;
        Ok(Self { index, source })
    }
}

impl<S> Drop for Watcher<S>
where
    S: event::Source,
{
    fn drop(&mut self) {
        REACTOR
            .registry
            .deregister(&mut self.source)
            .expect("fail to deregister source");
        REACTOR
            .wakers
            .write()
            .expect("entry lock poisoned")
            .remove(self.index);
    }
}
