use super::util::may_block;
use crossbeam_queue::SegQueue;
use mio::event;
use mio::{Events, Interest, Poll, Registry, Token};
use once_cell::sync::Lazy;
use slab::Slab;
use std::io;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Waker;
use std::task::{self, Context};
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

impl Reactor {
    fn read(&self, token: Token, waker: Waker) {
        let wakers = self.wakers.read().expect("entry lock poisoned");
        wakers[token.0].reader.push(waker);
    }

    fn write(&self, token: Token, waker: Waker) {
        let wakers = self.wakers.read().expect("entry lock poisoned");
        wakers[token.0].writer.push(waker);
    }
}

pub struct Watcher<S>
where
    S: event::Source,
{
    pub(crate) token: Token,
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
        let token = Token(index);
        REACTOR
            .registry
            .register(&mut source, token, ALL_INTEREST)?;
        Ok(Self { token, source })
    }

    pub fn poll_read_with<'a, F, R>(
        &'a self,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> task::Poll<io::Result<R>>
    where
        F: FnMut(&'a S) -> io::Result<R>,
    {
        REACTOR.read(self.token, cx.waker().clone());
        may_block(f(&self.source))
    }

    pub fn poll_write_with<'a, F, R>(
        &'a self,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> task::Poll<io::Result<R>>
    where
        F: FnMut(&'a S) -> io::Result<R>,
    {
        REACTOR.write(self.token, cx.waker().clone());
        may_block(f(&self.source))
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
            .remove(self.token.0);
    }
}

impl<S> Deref for Watcher<S>
where
    S: event::Source,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}
