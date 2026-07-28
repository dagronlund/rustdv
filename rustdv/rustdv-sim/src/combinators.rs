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
///
/// The futures are boxed behind lifetime `'a` rather than `'static` (D82), so
/// a future that *borrows* — a component's `run` borrowing the tree, a
/// sub-sequence borrowing its parent sequence — can be composed here. `'static`
/// futures satisfy any `'a`, so every earlier caller is unaffected.
pub struct First2<'a, A, B> {
    a: Pin<Box<dyn Future<Output = A> + 'a>>,
    b: Pin<Box<dyn Future<Output = B> + 'a>>,
}

pub fn first2<'a, FA, FB>(a: FA, b: FB) -> First2<'a, FA::Output, FB::Output>
where
    FA: Future + 'a,
    FB: Future + 'a,
{
    First2 { a: Box::pin(a), b: Box::pin(b) }
}

impl<A, B> Future for First2<'_, A, B> {
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

/// Join of two futures (port of `Combine`; SystemVerilog's `fork...join`).
pub struct Join2<'a, A, B> {
    a: Pin<Box<dyn Future<Output = A> + 'a>>,
    b: Pin<Box<dyn Future<Output = B> + 'a>>,
    ra: Option<A>,
    rb: Option<B>,
}

pub fn join2<'a, FA, FB>(a: FA, b: FB) -> Join2<'a, FA::Output, FB::Output>
where
    FA: Future + 'a,
    FB: Future + 'a,
{
    Join2 { a: Box::pin(a), b: Box::pin(b), ra: None, rb: None }
}

impl<A, B> Future for Join2<'_, A, B> {
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
pub async fn with_timeout<'a, F>(fut: F, d: SimDuration) -> Result<F::Output, TimeoutError>
where
    F: Future + 'a,
{
    match first2(fut, Timer::new(d)).await {
        Either::First(v) => Ok(v),
        Either::Second(()) => Err(TimeoutError),
    }
}

/// Join *N* futures, where N is known only at run time (D82).
///
/// `join2` covers the fixed-arity case the `join!` macro expands to; this
/// covers a `Vec` built at run time — a parent joining however many children
/// it has, or a virtual sequence joining a list of sub-sequences. Every future
/// is polled on each wake until all have completed; results come back in the
/// original order. Like `Join2`, the futures may borrow (`'a`).
pub struct JoinAll<'a, T> {
    futs: Vec<Option<Pin<Box<dyn Future<Output = T> + 'a>>>>,
    out: Vec<Option<T>>,
}

pub fn join_all<'a, T>(futs: Vec<Pin<Box<dyn Future<Output = T> + 'a>>>) -> JoinAll<'a, T> {
    let n = futs.len();
    JoinAll { futs: futs.into_iter().map(Some).collect(), out: (0..n).map(|_| None).collect() }
}

impl<T> Future for JoinAll<'_, T> {
    type Output = Vec<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Vec<T>> {
        // Sound: the inner futures are boxed (they own their pinning), and
        // `out` holds plain values we move out once every future is done.
        let this = unsafe { self.get_unchecked_mut() };
        let mut all_done = true;
        for (i, slot) in this.futs.iter_mut().enumerate() {
            if let Some(f) = slot {
                match f.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        this.out[i] = Some(v);
                        *slot = None; // drop the finished future
                    }
                    Poll::Pending => all_done = false,
                }
            }
        }
        if all_done {
            Poll::Ready(this.out.iter_mut().map(|o| o.take().unwrap()).collect())
        } else {
            Poll::Pending
        }
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
