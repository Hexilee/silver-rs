use futures::Stream;
use futures_timer::Delay;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// This function is an async version of [`std::thread::sleep`].
///
/// [`std::thread::sleep`]: https://doc.rust-lang.org/std/thread/fn.sleep.html
///
/// See also: [`interval`].
///
/// [`interval`]: fn.interval.html
///
/// # Examples
///
/// ```
/// # tio::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use tio::task;
///
/// task::sleep(Duration::from_secs(1)).await;
/// #
/// # })
/// ```
#[cfg_attr(feature = "docs", doc(cfg(feature = "timer")))]
#[inline]
pub async fn sleep(dur: Duration) {
    Delay::new(dur).await
}

/// A stream representing notifications at fixed interval
///
/// This stream is created by the [`interval`] function. See its
/// documentation for more.
///
/// [`interval`]: fn.interval.html
#[cfg_attr(feature = "docs", doc(cfg(feature = "timer")))]
pub struct Interval {
    dur: Duration,
    delay: Delay,
}

/// Creates a new stream that yields at a set interval.
///
/// The stream first yields after `dur`, and continues to yield every
/// `dur` after that. The stream accounts for time elapsed between calls, and
/// will adjust accordingly to prevent time skews.
///
/// Each interval may be slightly longer than the specified duration, but never
/// less.
///
/// Note that intervals are not intended for high resolution timers, but rather
/// they will likely fire some granularity after the exact instant that they're
/// otherwise indicated to fire at.
///
/// See also: [`sleep`].
///
/// [`sleep`]: fn.sleep.html
///
/// # Examples
///
/// Basic example:
///
/// ```no_run
/// use futures::prelude::*;
/// use tio::task;
/// use std::time::Duration;
///
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// let mut interval = task::interval(Duration::from_secs(4));
/// while let Some(_) = interval.next().await {
///     println!("prints every four seconds");
/// }
/// #
/// # Ok(()) }) }
/// ```
#[cfg_attr(feature = "docs", doc(cfg(feature = "timer")))]
#[inline]
pub fn interval(dur: Duration) -> Interval {
    Interval {
        dur,
        delay: Delay::new(dur),
    }
}

impl Stream for Interval {
    type Item = ();
    #[inline]
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.delay).poll(cx) {
            Poll::Ready(()) => {
                let dur = self.dur;
                self.delay.reset(dur);
                Poll::Ready(Some(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::task;
    use futures::prelude::*;
    use std::time::{Duration, Instant};

    #[test]
    fn interval() -> std::io::Result<()> {
        task::block_on(async {
            let gap = Duration::from_secs(1);
            let times = 5;
            let mut interval = task::interval(gap);
            let mut counter = 0;
            let start = Instant::now();
            while let Some(_) = interval.next().await {
                counter += 1;
                if counter >= times {
                    break;
                }
            }
            assert_eq!(5, counter);
            assert!(start.elapsed() >= times * gap);
            Ok(())
        })
    }
}
