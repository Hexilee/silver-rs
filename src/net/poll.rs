use mio::event;
use mio::{Events, Interest, Poll, Registry, Token};
use once_cell::sync::Lazy;
use slab::Slab;
use std::io;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;

const EVENTS: usize = 1 << 12;
const THREAD_NAME: &str = "tio/poll";
const ALL_INTEREST: Interest = {
    let mut interest = Interest::READABLE.add(Interest::WRITABLE);
    #[cfg(any(
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "macos"
    ))]
    {
        interest = interest.add(Interest::AIO)
    }
    #[cfg(target_os = "freebsd")]
    {
        interest = interest.add(Interest::LIO)
    }
    interest
};

static REACTOR: Lazy<Reactor> = Lazy::new(|| {
    let mut poll = match Poll::new() {
        Ok(p) => p,
        Err(err) => panic!("fail to construct a mio::Poll: {}", err),
    };
    let registry = match poll.registry().try_clone() {
        Ok(r) => r,
        Err(err) => panic!("fail to clone mio::Registry: {}", err),
    };
    thread::Builder::new()
        .name(THREAD_NAME.to_string())
        .spawn(move || {
            let mut events = Events::with_capacity(EVENTS);
            loop {
                if let Err(err) = poll.poll(&mut events, None) {
                    panic!("poll error: {}", err)
                }
            }
        })
        .expect(&format!("fail to spawn thread {}", THREAD_NAME));
    Reactor {
        registry,
        entries: RwLock::new(Slab::new()),
    }
});

struct Reactor {
    registry: Registry,
    entries: RwLock<Slab<Entry>>,
}

struct Watcher<S>
where
    S: event::Source,
{
    is_watching: bool,
    source: Arc<Source<S>>,
}

struct Source<S>
where
    S: event::Source,
{
    index: usize,
    source: S,
}

struct Entry {}

impl Entry {
    fn new() -> Self {
        Entry {}
    }
}

impl<S> Watcher<S>
where
    S: event::Source,
{
    fn new(source: Source<S>) -> Self {
        Self {
            is_watching: false,
            source: Arc::new(source),
        }
    }
}

impl<S> Clone for Watcher<S>
where
    S: event::Source,
{
    fn clone(&self) -> Self {
        Self {
            is_watching: false,
            source: self.source.clone(),
        }
    }
}

impl<S> Source<S>
where
    S: event::Source,
{
    fn new(mut source: S) -> io::Result<Self> {
        let index = REACTOR
            .entries
            .write()
            .expect("entry lock poisoned")
            .insert(Entry::new());
        REACTOR
            .registry
            .register(&mut source, Token(index), ALL_INTEREST)?;
        Ok(Self { index, source })
    }
}

impl<S> Drop for Source<S>
where
    S: event::Source,
{
    fn drop(&mut self) {
        REACTOR
            .registry
            .deregister(&mut self.source)
            .expect("fail to deregister source");
        REACTOR
            .entries
            .write()
            .expect("entry lock poisoned")
            .remove(self.index);
    }
}
