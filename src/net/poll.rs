use super::util::{may_block, ENTRIES_LOCK_POISONED};
use crossbeam_queue::SegQueue;
use futures::future::{join, poll_fn};
use mio::event;
use mio::{Events, Interest, Poll, Registry, Token};
use once_cell::sync::Lazy;
use slab::Slab;
use std::fmt::{self, Debug, Formatter};
use std::io;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
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
    let entries = Arc::new(RwLock::new(Slab::<Entry>::new()));
    let entries_cloned = entries.clone();
    thread::Builder::new()
        .name(THREAD_NAME.to_string())
        .spawn(move || {
            let mut events = Events::with_capacity(EVENTS);
            loop {
                if let Err(err) = poll.poll(&mut events, None) {
                    panic!("poll error: {}", err)
                }
                let entries = entries_cloned.read().expect(ENTRIES_LOCK_POISONED);
                for event in events.iter() {
                    let token = event.token();
                    let entry = &entries[token.0];
                    if event.is_readable() {
                        entry.reader.ready();
                    }
                    if event.is_writable() {
                        entry.writer.ready();
                    }
                }
            }
        })
        .expect(&format!("fail to spawn thread {}", THREAD_NAME));
    Reactor { registry, entries }
});

struct Reactor {
    registry: Registry,
    entries: Arc<RwLock<Slab<Entry>>>,
}

impl Reactor {
    #[inline]
    fn entries(&self) -> RwLockReadGuard<Slab<Entry>> {
        self.entries.read().expect(ENTRIES_LOCK_POISONED)
    }

    #[inline]
    fn entries_mut(&self) -> RwLockWriteGuard<Slab<Entry>> {
        self.entries.write().expect(ENTRIES_LOCK_POISONED)
    }

    #[inline]
    fn read(&self, token: Token, waker: Waker) {
        self.entries()[token.0].reader.wakers.push(waker);
    }

    #[inline]
    fn write(&self, token: Token, waker: Waker) {
        self.entries()[token.0].writer.wakers.push(waker);
    }
}

pub struct Watcher<S>
where
    S: event::Source,
{
    pub(crate) token: Token,
    pub(crate) source: S,
}

struct Entry {
    reader: Channel,
    writer: Channel,
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
            reader: Channel::new(),
            writer: Channel::new(),
        }
    }
}

impl<S> Watcher<S>
where
    S: event::Source,
{
    pub fn new(mut source: S) -> Self {
        let index = REACTOR.entries_mut().insert(Entry::new());
        let token = Token(index);
        REACTOR
            .registry
            .register(&mut source, token, ALL_INTEREST)
            .expect("fail to register source");
        Self { token, source }
    }

    #[inline]
    pub async fn ready(&self) {
        join(self.read_ready(), self.write_ready()).await;
    }

    #[inline]
    pub async fn read_ready(&self) {
        poll_fn(|cx| {
            let channel = &REACTOR.entries()[self.token.0].reader;
            if !channel.is_ready() {
                REACTOR.read(self.token, cx.waker().clone());
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
    pub async fn write_ready(&self) {
        poll_fn(|cx| {
            let channel = &REACTOR.entries()[self.token.0].writer;
            if !channel.is_ready() {
                REACTOR.write(self.token, cx.waker().clone());
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
            REACTOR.read(self.token, cx.waker().clone());
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
            REACTOR.write(self.token, cx.waker().clone());
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
        REACTOR.entries_mut().remove(self.token.0);
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
            .field("token", &self.token)
            .field("source", &self.source)
            .finish()
    }
}

// mod mock_lock {
//     use std::error::Error;
//     use std::ops::{Deref, DerefMut};
//     use std::result::Result as StdResult;
//     use std::sync;
//
//     pub struct RwLock<T>(sync::RwLock<T>);
//     pub struct RwLockReadGuard<'a, T: 'a>(sync::RwLockReadGuard<'a, T>, &'static str);
//     pub struct RwLockWriteGuard<'a, T: 'a>(sync::RwLockWriteGuard<'a, T>, &'static str);
//
//     type Result<T> = StdResult<T, Box<dyn 'static + Error>>;
//
//     impl<T> RwLock<T> {
//         pub fn new(val: T) -> Self {
//             Self(sync::RwLock::new(val))
//         }
//
//         pub fn read(&self, caller: &'static str) -> Result<RwLockReadGuard<'_, T>> {
//             println!("{} try read lock", caller);
//             let guard = RwLockReadGuard(self.0.read().unwrap(), caller);
//             println!("{} read lock", caller);
//             Ok(guard)
//         }
//
//         pub fn write(&self, caller: &'static str) -> Result<RwLockWriteGuard<'_, T>> {
//             println!("{} try write lock", caller);
//             let guard = RwLockWriteGuard(self.0.write().unwrap(), caller);
//             println!("{} write lock", caller);
//             Ok(guard)
//         }
//     }
//
//     impl<'a, T> Deref for RwLockReadGuard<'a, T> {
//         type Target = sync::RwLockReadGuard<'a, T>;
//         fn deref(&self) -> &Self::Target {
//             &self.0
//         }
//     }
//
//     impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
//         type Target = sync::RwLockWriteGuard<'a, T>;
//         fn deref(&self) -> &Self::Target {
//             &self.0
//         }
//     }
//
//     impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
//         fn deref_mut(&mut self) -> &mut Self::Target {
//             &mut self.0
//         }
//     }
//
//     impl<'a, T> Drop for RwLockReadGuard<'a, T> {
//         fn drop(&mut self) {
//             println!("{} read unlock", self.1)
//         }
//     }
//
//     impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
//         fn drop(&mut self) {
//             println!("{} write unlock", self.1)
//         }
//     }
// }
