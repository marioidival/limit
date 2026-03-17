//! AgentActor — wraps a [`TeamAgent`] in the [`Actor`](super::actor::Actor) trait.
//!
//! Each role (PM, TL, Jr) gets its own `AgentActor` instance running in a
//! separate tokio task. The actor receives [`TeamMessage`] variants, calls
//! the underlying `TeamAgent::prompt()`, and sends replies via oneshot channels.

use super::actor::Actor;
use super::agent::PromptResult;
use super::messages::{TeamMessage, MAX_HISTORY_PER_AGENT, MAX_TASKS};
use crate::error::AgentError;
use crate::team::agent::TeamAgent;
use crate::team::history::{EventLevel, TeamEvent, TeamHistory};
use crate::team::orchestrator::TaskResult;
use crate::team::role::Role;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// An actor wrapping a [`TeamAgent`] that processes [`TeamMessage`]s.
pub struct AgentActor {
    role: Role,
    agent: TeamAgent,
    history: Arc<RwLock<TeamHistory>>,
    progress_tx: Option<mpsc::UnboundedSender<crate::team::progress::TeamProgressEvent>>,
    /// Shared counter for accumulating input tokens across prompts.
    token_input: Arc<AtomicU64>,
    /// Shared counter for accumulating output tokens across prompts.
    token_output: Arc<AtomicU64>,
}

impl AgentActor {
    pub fn new(
        role: Role,
        agent: TeamAgent,
        history: Arc<RwLock<TeamHistory>>,
        progress_tx: Option<mpsc::UnboundedSender<crate::team::progress::TeamProgressEvent>>,
        token_input: Arc<AtomicU64>,
        token_output: Arc<AtomicU64>,
    ) -> Self {
        Self {
            role,
            agent,
            history,
            progress_tx,
            token_input,
            token_output,
        }
    }

    async fn log_event(&self, action: &str, content: &str) {
        let mut h = self.history.write().await;
        h.add_event(TeamEvent {
            timestamp: chrono::Utc::now(),
            role: self.role.label().to_string(),
            action: action.to_string(),
            content: content.to_string(),
            level: EventLevel::default(),
        });
    }

    async fn prompt(&mut self, input: &str) -> Result<PromptResult, AgentError> {
        let start = std::time::Instant::now();
        let result = self.agent.prompt(input).await;
        let elapsed = start.elapsed();

        match &result {
            Ok(pr) => {
                self.token_input
                    .fetch_add(pr.usage.input_tokens, Ordering::Relaxed);
                self.token_output
                    .fetch_add(pr.usage.output_tokens, Ordering::Relaxed);
                tracing::info!(
                    "[team] {:?} prompt completed in {:?} ({}in/{}out tokens{}), response: {} chars",
                    self.role,
                    elapsed,
                    pr.usage.input_tokens,
                    pr.usage.output_tokens,
                    if pr.hit_tool_limit { " [TOOL-LIMIT]" } else { "" },
                    pr.text.len(),
                );
            }
            Err(e) => {
                tracing::error!(
                    "[team] {:?} prompt failed after {:?}: {}",
                    self.role,
                    elapsed,
                    e
                );
            }
        }

        result
    }

    fn trim_history(&mut self) {
        self.agent.trim_history(MAX_HISTORY_PER_AGENT);
    }
}

impl Actor for AgentActor {
    type Message = TeamMessage;

    fn handle(
        &mut self,
        msg: TeamMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), AgentError>> + Send + '_>> {
        Box::pin(async move {
            match msg {
                TeamMessage::Shutdown => {
                    tracing::info!("[actor] {:?} received Shutdown", self.role);
                    Err(AgentError::ActorError(format!(
                        "{:?} received shutdown",
                        self.role
                    )))
                }

                TeamMessage::PmAnalyze { request, reply, .. } => {
                    tracing::info!("[actor] PM received PmAnalyze ({} chars)", request.len());
                    let result = self
                        .prompt(&format!(
                            "User request:\n{request}\n\nAnalyze this request and identify what needs to be done."
                        ))
                        .await;
                    self.log_event(
                        "analysis",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::PmDeliver {
                    validation,
                    results_summary,
                    files_modified,
                    reply,
                } => {
                    tracing::info!(
                        "[actor] PM received PmDeliver ({} files)",
                        files_modified.len()
                    );
                    let files_list = if files_modified.is_empty() {
                        "No files modified.".to_string()
                    } else {
                        files_modified
                            .iter()
                            .map(|f| format!("- {f}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let result = self
                        .prompt(&format!(
                            "Technical solution validated by TL:\n{validation}\n\n\
                             Task results:\n{results_summary}\n\n\
                             Files modified:\n{files_list}\n\n\
                             Summarize what was done: list the files created/modified and the outcome. \
                             Only suggest follow-up actions if something is genuinely incomplete or broken. \
                             Do NOT suggest improvements, refactors, or enhancements."
                        ))
                        .await;
                    self.log_event(
                        "delivery",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::PmDeliverNoTasks { plan, reply, .. } => {
                    tracing::info!(
                        "[actor] PM received PmDeliverNoTasks ({} chars)",
                        plan.len()
                    );
                    let result = self
                        .prompt(&format!(
                            "The Tech Lead produced a plan but no specific tasks. Here is the plan:\n\n{plan}\n\n\
                             Summarize this for the user and suggest next steps."
                        ))
                        .await;
                    self.log_event(
                        "delivery",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::TlPlan { analysis, reply } => {
                    tracing::info!("[actor] TL received TlPlan ({} chars)", analysis.len());
                    let result = self
                        .prompt(&format!(
                            "PM analysis:\n{analysis}\n\nCreate a technical plan to implement this."
                        ))
                        .await;
                    self.log_event(
                        "plan",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::TlBreakdown { plan, reply } => {
                    tracing::info!("[actor] TL received TlBreakdown ({} chars)", plan.len());
                    let result = self
                        .prompt(&format!(
                            "Technical plan:\n{plan}\n\nBreak this down into at most {MAX_TASKS} specific, \
                             executable tasks. Each task should be self-contained and independently \
                             completable by a junior developer. Combine small steps into single tasks.\n\n\
                             IMPORTANT: Do NOT read any files or use tools. Do NOT include CONTEXT blocks. \
                             Junior agents have their own tools to read files — just describe what to do.\n\n\
                             If a task depends on the output of another task, add DEPENDS_ON on the next line:\n\
                             TASK: <dependent task>\nDEPENDS_ON: <task it depends on>\n\n\
                             CRITICAL: Count every distinct deliverable in the plan. Each one MUST have a TASK. \
                             Never skip a deliverable. Double-check your task list against the plan before outputting.\n\n\
                             Output ONLY the task list, starting with TASK: on each line."
                        ))
                        .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::TlValidate {
                    results_summary,
                    files_list,
                    reply,
                } => {
                    tracing::info!(
                        "[actor] TL received TlValidate ({} chars, {} chars files list)",
                        results_summary.len(),
                        files_list.len()
                    );
                    let result = self
                        .prompt(&format!(
                            "Task results:\n{results_summary}\n\n\
                             {files_list}\n\n\
                             Validate the implementation based on the task results above. \
                             Do NOT use tools — just analyze the results and note any issues."
                        ))
                        .await;
                    self.log_event(
                        "validation",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::JrExecute { task, reply } => {
                    let task_id = task.id.clone();
                    let desc_preview: String = task
                        .description
                        .lines()
                        .next()
                        .unwrap_or(&task.description)
                        .chars()
                        .take(60)
                        .collect();
                    tracing::info!(
                        "[actor] Jr received task {} — {:.60}…",
                        task_id,
                        desc_preview
                    );
                    let prompt_text = format!(
                        "Execute this task:\n{}\n\n\
                         Do the work efficiently. Use the minimum number of tool calls needed (aim for 1-3). \
                         Do not verify or re-read files after writing them — trust the tool results. \
                         If CONTEXT is provided in the task, use it directly instead of reading the file. \
                         Do not run ls/cat/echo to verify your work.",
                        task.description
                    );

                    let task_result = match self.prompt(&prompt_text).await {
                        Ok(pr) => TaskResult {
                            task_id,
                            output: pr.text,
                            success: true,
                            hit_tool_limit: pr.hit_tool_limit,
                            files_modified: pr.files_modified,
                        },
                        Err(e) => TaskResult {
                            task_id,
                            output: format!("Task failed: {}", e),
                            success: false,
                            hit_tool_limit: false,
                            files_modified: vec![],
                        },
                    };

                    self.trim_history();
                    let _ = reply.send(task_result);
                    Ok(())
                }
            }
        })
    }
}
