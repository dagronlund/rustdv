//! Ports of `First`/`Combine` (design-doc mapping row 16): future
//! combinators with drop-based cancellation of the losers — the cleanup
//! cocotb does manually with kill-on-completion tasks falls out of RAII.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::time::SimDuration;
use crate::triggers::Timer;

pub enum Either<A, B> {
    First(A),
    Second(B),
}

/// First of two futures; the loser is dropped (unsubscribing its trigger).
pub struct First2<A, B> {
    a: Pin<Box<dyn Future<Output = A>>>,
    b: Pin<Box<dyn Future<Output = B>>>,
}

pub fn first2<FA, FB>(a: FA, b: FB) -> First2<FA::Output, FB::Output>
where
    FA: Future + 'static,
    FB: Future + 'static,
{
    First2 { a: Box::pin(a), b: Box::pin(b) }
}

impl<A, B> Future for First2<A, B> {
    type Output = Either<A, B>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = self.a.as_mut().poll(cx) {
            return Poll::Ready(Either::First(v));
        }
        if let Poll::Ready(v) = self.b.as_mut().poll(cx) {
            return Poll::Ready(Either::Second(v));
        }
        Poll::Pending
    }
}

/// Join of two futures (port of `Combine`).
pub struct Join2<A, B> {
    a: Pin<Box<dyn Future<Output = A>>>,
    b: Pin<Box<dyn Future<Output = B>>>,
    ra: Option<A>,
    rb: Option<B>,
}

pub fn join2<FA, FB>(a: FA, b: FB) -> Join2<FA::Output, FB::Output>
where
    FA: Future + 'static,
    FB: Future + 'static,
{
    Join2 { a: Box::pin(a), b: Box::pin(b), ra: None, rb: None }
}

impl<A, B> Future for Join2<A, B> {
    type Output = (A, B);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<(A, B)> {
        // Sound: the inner futures are boxed (their pinning is their own),
        // and ra/rb are plain values we intentionally move on completion.
        let this = unsafe { self.get_unchecked_mut() };
        if this.ra.is_none() {
            if let Poll::Ready(v) = this.a.as_mut().poll(cx) {
                this.ra = Some(v);
            }
        }
        if this.rb.is_none() {
            if let Poll::Ready(v) = this.b.as_mut().poll(cx) {
                this.rb = Some(v);
            }
        }
        if this.ra.is_some() && this.rb.is_some() {
            Poll::Ready((this.ra.take().unwrap(), this.rb.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "operation timed out")
    }
}
impl std::error::Error for TimeoutError {}

/// Run `fut` with a simulation-time timeout.
pub async fn with_timeout<F>(fut: F, d: SimDuration) -> Result<F::Output, TimeoutError>
where
    F: Future + 'static,
{
    match first2(fut, Timer::new(d)).await {
        Either::First(v) => Ok(v),
        Either::Second(()) => Err(TimeoutError),
    }
}

/// `first!(a, b, ...)` — first completed future wins; losers are dropped.
#[macro_export]
macro_rules! first {
    ($a:expr, $b:expr $(,)?) => {
        $crate::combinators::first2($a, $b)
    };
    ($a:expr, $b:expr, $($rest:expr),+ $(,)?) => {
        $crate::combinators::first2($a, $crate::first!($b, $($rest),+))
    };
}

/// `join!(a, b, ...)` — wait for all.
#[macro_export]
macro_rules! join {
    ($a:expr, $b:expr $(,)?) => {
        $crate::combinators::join2($a, $b)
    };
    ($a:expr, $b:expr, $($rest:expr),+ $(,)?) => {
        $crate::combinators::join2($a, $crate::join!($b, $($rest),+))
    };
}
