use futures::task::AtomicWaker;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

/// A token that can be used to signal that we want to shutdown. The token can be cloned so it can
/// be passed around and it can be waited upon.
#[derive(Debug, Clone)]
pub struct ShutdownToken(Arc<(AtomicBool, AtomicWaker)>);

impl Default for ShutdownToken {
    fn default() -> Self {
        ShutdownToken(Arc::new((AtomicBool::new(false), Default::default())))
    }
}

impl ShutdownToken {
    /// Returns true if the token is indicating a shutdown
    pub fn is_awaiting_shutdown(&self) -> bool {
        (self.0).0.load(Ordering::Relaxed)
    }

    /// Tell the token to go to shutdown state
    pub fn shutdown(&self) {
        (self.0).0.store(true, Ordering::Relaxed);
        (self.0).1.wake();
    }
}

impl Future for ShutdownToken {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_awaiting_shutdown() {
            Poll::Ready(())
        } else {
            (self.0).1.register(cx.waker());

            if self.is_awaiting_shutdown() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}
