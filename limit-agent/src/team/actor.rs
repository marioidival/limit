//! Core actor framework — trait, mailbox, and spawn.
//!
//! Each agent (PM, TL, Jr) runs as an independent actor with its own
//! tokio task and `mpsc` mailbox. [`ActorRef`] is a cheap clone-able
//! handle for sending messages.

use crate::error::AgentError;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// An actor that processes messages of type `M`.
pub trait Actor: Send + 'static {
    type Message: Send + 'static;

    /// Handle a single message. Return `Err` to signal the actor
    /// should shut down (logged, not propagated to sender).
    fn handle(
        &mut self,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + '_>>;
}

/// Clone-able handle to an actor's mailbox.
pub struct ActorRef<T> {
    tx: mpsc::Sender<T>,
}

impl<T> Clone for ActorRef<T> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<T: Send> ActorRef<T> {
    /// Send a message to the actor's mailbox (async).
    ///
    /// Returns `Err(msg)` if the actor's task has exited (mailbox closed).
    pub async fn send(&self, msg: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(msg).await
    }
}

/// Spawn an actor as a tokio task with a bounded mailbox.
///
/// Returns an [`ActorRef`] for sending messages and a [`JoinHandle`]
/// that completes when the actor's mailbox closes or an error occurs.
pub fn spawn<A: Actor<Message = M>, M: Send + 'static>(
    mut actor: A,
    mailbox_size: usize,
) -> (ActorRef<M>, JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel::<M>(mailbox_size);

    let handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = actor.handle(msg).await {
                tracing::error!("[actor] actor shutting down: {}", e);
                break;
            }
        }
    });

    (ActorRef { tx }, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A simple counter actor for testing.
    struct CounterActor {
        count: Arc<AtomicUsize>,
    }

    impl CounterActor {
        fn new() -> (Self, Arc<AtomicUsize>) {
            let count = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    count: count.clone(),
                },
                count,
            )
        }
    }

    impl Actor for CounterActor {
        type Message = ();

        fn handle(
            &mut self,
            _msg: Self::Message,
        ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + '_>> {
            Box::pin(async move {
                self.count.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_spawn_and_send() {
        let (actor, count) = CounterActor::new();
        let (ref_, handle) = spawn(actor, 10);

        ref_.send(()).await.unwrap();
        ref_.send(()).await.unwrap();
        ref_.send(()).await.unwrap();

        drop(ref_);
        handle.await.unwrap();

        assert_eq!(count.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_actor_ref_clone() {
        let (actor, count) = CounterActor::new();
        let (ref_, handle) = spawn(actor, 10);

        let ref2 = ref_.clone();
        ref_.send(()).await.unwrap();
        ref2.send(()).await.unwrap();

        drop(ref_);
        drop(ref2);
        handle.await.unwrap();

        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_actor_error_shuts_down() {
        struct FailActor;

        impl Actor for FailActor {
            type Message = ();

            fn handle(
                &mut self,
                _msg: Self::Message,
            ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + '_>> {
                Box::pin(async { Err(AgentError::ActorError("boom".into())) })
            }
        }

        let (ref_, handle) = spawn(FailActor, 10);
        ref_.send(()).await.unwrap();
        handle.await.unwrap();

        // Mailbox closed after error
        assert!(ref_.send(()).await.is_err());
    }
}
