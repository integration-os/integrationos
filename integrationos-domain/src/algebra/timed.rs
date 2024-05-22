use futures::Future;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub trait TimedExt: Sized + Future {
    fn timed<F>(self, f: F) -> Timed<Self, F>
    where
        F: FnMut(&Self::Output, Duration),
    {
        Timed {
            inner: self,
            f,
            start: None,
        }
    }
}

#[pin_project]
pub struct Timed<Fut, F>
where
    Fut: Future,
    F: FnMut(&Fut::Output, Duration),
{
    #[pin]
    inner: Fut,
    f: F,
    start: Option<Instant>,
}

impl<Fut, F> Future for Timed<Fut, F>
where
    Fut: Future,
    F: FnMut(&Fut::Output, Duration),
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.project();

        match this.start {
            Some(start) => match this.inner.poll(cx) {
                Poll::Ready(output) => {
                    let elapsed = start.elapsed();
                    (this.f)(&output, elapsed);
                    Poll::Ready(output)
                }
                Poll::Pending => Poll::Pending,
            },
            None => {
                *this.start = Some(Instant::now());
                // Continue polling after setting the start time.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl<F: Future> TimedExt for F {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::ready;

    #[tokio::test]
    async fn test_timed() {
        let mut elapsed = None;
        let fut = ready(42).timed(|output, duration| {
            elapsed = Some(duration);
            assert_eq!(*output, 42);
        });
        assert_eq!(fut.await, 42);
        assert!(elapsed.is_some());
    }
}
