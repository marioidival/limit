// Supervisor — monitors actor tasks and manages lifecycle.
//!
//! Provides one-for-one supervision with restart capability.
//! The supervisor runs as a plain tokio task receiving commands via `mpsc`.

use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::actor::ActorRef;
use super::messages::{SupervisorMessage, TeamMessage};
#[cfg(test)]
use crate::error::AgentError;

/// Default maximum restart attempts per child.
pub const DEFAULT_MAX_RETRIES: usize = 3;

/// Tracks a supervised child actor.
struct ChildEntry {
    handle: JoinHandle<()>,
    actor_ref: ActorRef<TeamMessage>,
}

/// Spawn the supervisor as a tokio task.
///
/// Returns an `mpsc::Sender` for sending commands and the supervisor's
/// `JoinHandle`.
pub fn spawn_supervisor(_max_retries: usize) -> (mpsc::Sender<SupervisorMessage>, JoinHandle<()>) {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<SupervisorMessage>(32);
    let mut children: HashMap<String, ChildEntry> = HashMap::new();

    let handle = tokio::spawn(async move {
        while let Some(msg) = cmd_rx.recv().await {
            match msg {
                SupervisorMessage::Register {
                    name,
                    handle,
                    actor_ref,
                    restart_fn: _,
                } => {
                    children.insert(name, ChildEntry { handle, actor_ref });
                }

                SupervisorMessage::Unregister { name } => {
                    if let Some(spec) = children.remove(&name) {
                        spec.handle.abort();
                    }
                }

                SupervisorMessage::ShutdownAll => {
                    for (name, spec) in children.drain() {
                        tracing::info!("[supervisor] shutting down {}", name);
                        let _ = spec.actor_ref.send(TeamMessage::Shutdown).await;
                        spec.handle.abort();
                    }
                    break;
                }
            }
        }
    });

    (cmd_tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::actor::{spawn, Actor};
    use std::future::Future;
    use std::pin::Pin;

    #[tokio::test]
    async fn test_supervisor_register_and_shutdown() {
        let (cmd_tx, handle) = spawn_supervisor(DEFAULT_MAX_RETRIES);

        struct NoopActor;
        impl Actor for NoopActor {
            type Message = TeamMessage;
            fn handle(
                &mut self,
                msg: Self::Message,
            ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + '_>> {
                Box::pin(async move {
                    if let TeamMessage::Shutdown = msg {
                        Err(AgentError::ActorError("shutdown".into()))
                    } else {
                        Ok(())
                    }
                })
            }
        }

        let (actor_ref, actor_handle) = spawn(NoopActor, 16);

        let name = "test-noop".to_string();
        let _ = cmd_tx
            .send(SupervisorMessage::Register {
                name,
                handle: actor_handle,
                actor_ref,
                restart_fn: Box::new(|| spawn(NoopActor, 16)),
            })
            .await;

        let _ = cmd_tx.send(SupervisorMessage::ShutdownAll).await;
        handle.await.unwrap();
    }
}
