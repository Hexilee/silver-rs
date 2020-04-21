use super::util::{may_block, ENTRIES_LOCK_POISONED};
use crossbeam_queue::SegQueue;
use futures::future::poll_fn;
use mio::event;
use mio::{Events, Interest, Poll, Registry, Token};
use once_cell::sync::Lazy;
use slab::Slab;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Waker;
use std::task::{self, Context};
use std::thread;

const EVENTS: usize = 1 << 12;
const THREAD_NAME: &str = "tio/poll";
const ALL_INTEREST: Interest = Interest::READABLE.add(Interest::WRITABLE);

static REACTOR: Lazy<Reactor> = Lazy::new(|| {
    let mut poll = Poll::new()
        .unwrap_or_else(|err| panic!("fail to construct a mio::Poll: {}", err));
    let registry = poll
        .registry()
        .try_clone()
        .map(Arc::new)
        .unwrap_or_else(|err| panic!("fail to clone mio::Registry: {}", err));
    let entries = Arc::new(RwLock::new(Slab::<Entry>::new()));
    let ret = Reactor { registry, entries };
    let reactor = ret.clone();
    thread::Builder::new()
        .name(THREAD_NAME.to_string())
        .spawn(move || {
            let mut events = Events::with_capacity(EVENTS);
            reactor.poll(&mut poll, &mut events);
        })
        .unwrap_or_else(|err| panic!("fail to spawn thread {}: {}", THREAD_NAME, err));
    ret
});

#[derive(Clone)]
struct Reactor {
    registry: Arc<Registry>,
    entries: Arc<RwLock<Slab<Entry>>>,
}

impl Reactor {
    #[inline]
    fn entry(&self, index: usize) -> Option<Entry> {
        self.entries
            .read()
            .expect(ENTRIES_LOCK_POISONED)
            .get(index)
            .cloned()
    }

    #[inline]
    fn insert(&self, entry: Entry) -> usize {
        self.entries
            .write()
            .expect(ENTRIES_LOCK_POISONED)
            .insert(entry)
    }

    #[inline]
    fn remove(&self, index: usize) -> Entry {
        self.entries
            .write()
            .expect(ENTRIES_LOCK_POISONED)
            .remove(index)
    }

    fn poll(&self, poll: &mut Poll, events: &mut Events) {
        loop {
            if let Err(err) = poll.poll(events, None) {
                log::error!("poll error: {}", err)
            } else {
                for event in events.iter() {
                    let token = event.token();
                    if let Some(entry) = self.entry(token.0) {
                        if event.is_readable() {
                            entry.reader.ready();
                        }
                        if event.is_writable() {
                            entry.writer.ready();
                        }
                    }
                }
            }
        }
    }
}

pub struct Watcher<S>
where
    S: event::Source,
{
    pub(crate) entry: Entry,
    pub(crate) index: usize,
    pub(crate) source: S,
}

#[derive(Clone)]
pub struct Entry {
    reader: Arc<Channel>,
    writer: Arc<Channel>,
}

struct Channel {
    ready: AtomicBool,
    wakers: SegQueue<Waker>,
}

impl Channel {
    #[inline]
    fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            wakers: SegQueue::new(),
        }
    }

    #[inline]
    fn ready(&self) {
        if !self.is_ready() {
            self.ready.store(true, Ordering::Relaxed)
        }
        while let Ok(waker) = self.wakers.pop() {
            waker.wake()
        }
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }
}

impl Entry {
    #[inline]
    fn new() -> Self {
        Entry {
            reader: Arc::new(Channel::new()),
            writer: Arc::new(Channel::new()),
        }
    }

    #[inline]
    fn read(&self, waker: Waker) {
        self.reader.wakers.push(waker);
    }

    #[inline]
    fn write(&self, waker: Waker) {
        self.writer.wakers.push(waker);
    }
}

impl<S> Watcher<S>
where
    S: event::Source,
{
    pub fn new(mut source: S) -> Self {
        let entry = Entry::new();
        let index = REACTOR.insert(entry.clone());
        REACTOR
            .registry
            .register(&mut source, Token(index), ALL_INTEREST)
            .expect("fail to register source");
        Self {
            entry,
            index,
            source,
        }
    }

    // ## Unused
    //
    // #[inline]
    // pub async fn ready(&self) {
    //     join(self.read_ready(), self.write_ready()).await;
    // }
    //
    // #[inline]
    // pub async fn read_ready(&self) {
    //     poll_fn(|cx| {
    //         let channel = &*self.entry.reader;
    //         if !channel.is_ready() {
    //             self.entry.read(cx.waker().clone());
    //         }
    //         if channel.is_ready() {
    //             task::Poll::Ready(())
    //         } else {
    //             task::Poll::Pending
    //         }
    //     })
    //     .await
    // }

    #[inline]
    pub async fn write_ready(&self) {
        poll_fn(|cx| {
            let channel = &*self.entry.writer;
            if !channel.is_ready() {
                self.entry.write(cx.waker().clone());
            }
            if channel.is_ready() {
                task::Poll::Ready(())
            } else {
                task::Poll::Pending
            }
        })
        .await
    }

    #[inline]
    pub fn poll_read_with<'a, F, R>(
        &'a self,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> task::Poll<io::Result<R>>
    where
        F: FnMut(&'a S) -> io::Result<R>,
    {
        let mut poll = may_block(f(&self.source));
        if poll.is_pending() {
            self.entry.read(cx.waker().clone());
            poll = may_block(f(&self.source));
        }
        poll
    }

    #[inline]
    pub fn poll_write_with<'a, F, R>(
        &'a self,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> task::Poll<io::Result<R>>
    where
        F: FnMut(&'a S) -> io::Result<R>,
    {
        let mut poll = may_block(f(&self.source));
        if poll.is_pending() {
            self.entry.write(cx.waker().clone());
            poll = may_block(f(&self.source));
        }
        poll
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
        REACTOR.remove(self.index);
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

impl<S> Debug for Watcher<S>
where
    S: event::Source + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Watcher")
            .field("index", &self.index)
            .field("source", &self.source)
            .finish()
    }
}
