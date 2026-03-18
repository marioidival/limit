//! AgentActor — wraps a [`TeamAgent`] in the [`Actor`](super::actor::Actor) trait.
//!
//! Each role (PM, TL, Jr) gets its own `AgentActor` instance running in a
//! separate tokio task. The actor receives [`TeamMessage`] variants, calls
//! the underlying `TeamAgent::prompt()`, and sends replies via oneshot channels.

use super::actor::Actor;
use super::agent::PromptResult;
use super::messages::{send_progress, TeamMessage, MAX_HISTORY_PER_AGENT, MAX_TASKS};
use crate::error::AgentError;
use crate::team::agent::TeamAgent;
use crate::team::history::{EventLevel, TeamEvent, TeamHistory};
use crate::team::orchestrator::TaskResult;
use crate::team::progress::TeamProgressEvent;
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
    /// Shared counter for accumulating input tokens across prompts.
    token_input: Arc<AtomicU64>,
    /// Shared counter for accumulating output tokens across prompts.
    token_output: Arc<AtomicU64>,
    /// Progress channel for emitting task sub-status updates.
    progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
}

impl AgentActor {
    pub fn new(
        role: Role,
        agent: TeamAgent,
        history: Arc<RwLock<TeamHistory>>,
        token_input: Arc<AtomicU64>,
        token_output: Arc<AtomicU64>,
        progress_tx: Option<mpsc::UnboundedSender<TeamProgressEvent>>,
    ) -> Self {
        Self {
            role,
            agent,
            history,
            token_input,
            token_output,
            progress_tx,
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

    /// Extract file/tool information from text for sub-status messages.
    fn extract_tool_info(text: &str) -> String {
        // Look for file paths in common patterns
        let patterns = [
            "editing ",
            "Reading ",
            "Writing ",
            "file: ",
            "src/",
            "Creating ",
            "Updating ",
        ];

        for pattern in &patterns {
            if let Some(pos) = text.find(pattern) {
                let after = &text[pos + pattern.len()..];
                // Extract the file path (up to 50 chars)
                let file = after
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(50)
                    .collect::<String>();
                if !file.is_empty() {
                    return format!("{}{}...", pattern.trim(), file);
                }
            }
        }

        // Fallback: first line of description
        text.lines().next().unwrap_or("").chars().take(40).collect()
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
                            "PM analysis:\n{analysis}\n\nCreate a detailed technical plan to implement this. \
                             Output the plan directly — do NOT say you will explore, read files, or investigate. \
                             You have no tools. Produce the plan now based on the analysis above."
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
                             For EACH task, include a DEFINITION_OF_DONE line with 2-4 concrete acceptance criteria.\n\
                             Format:\n\
                             TASK: <description>\n\
                             DEFINITION_OF_DONE:\n\
                             - <specific, verifiable criterion>\n\
                             - <e.g., \"file compiles without errors\">\n\
                             - <e.g., \"module exports the required public API\">\n\n\
                             If a task depends on the output of another task, add DEPENDS_ON on the next line:\
                             \nTASK: <dependent task>\nDEPENDS_ON: <task it depends on>\n\n\
                             CRITICAL: Count every distinct deliverable in the plan. Each one MUST have a TASK. \
                             Never skip a deliverable. Double-check your task list against the plan before outputting.\n\n\
                             Output ONLY the task list, starting with TASK: on each line."
                        ))
                        .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::TlSuggestBuildCommand {
                    files_modified,
                    reply,
                } => {
                    tracing::info!(
                        "[actor] TL received TlSuggestBuildCommand ({} files)",
                        files_modified.len()
                    );
                    let files_list = if files_modified.is_empty() {
                        "No files were modified.".to_string()
                    } else {
                        files_modified
                            .iter()
                            .map(|f| format!("- {f}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    };
                    let result = self
                        .prompt(&format!(
                            "Files modified during task execution:\n{files_list}\n\n\
                             Based on the project structure, suggest a single shell command to verify \
                             that the changes compile/build correctly.\n\n\
                             Output ONLY the command, nothing else. No explanation, no markdown, no code blocks.\n\
                             If no build system is detected or files weren't modified, output: NONE"
                        ))
                        .await;
                    self.log_event(
                        "build_command",
                        &result.as_ref().map(|r| r.text.clone()).unwrap_or_default(),
                    )
                    .await;
                    let _ = reply.send(result.map(|r| r.text));
                    Ok(())
                }

                TeamMessage::TlValidate {
                    results_summary,
                    files_list,
                    build_output,
                    reply,
                } => {
                    tracing::info!(
                        "[actor] TL received TlValidate ({} chars, {} chars files list)",
                        results_summary.len(),
                        files_list.len()
                    );
                    let build_section = match &build_output {
                        Some(output) => {
                            let trimmed = if output.len() > 3000 {
                                format!("{}...\n[truncated]", &output[..3000])
                            } else {
                                output.clone()
                            };
                            format!("\n\nBUILD STATUS: FAILED\n```\n{trimmed}\n```\n",)
                        }
                        None => "\n\nBUILD STATUS: PASSED".to_string(),
                    };
                    let result = self
                        .prompt(&format!(
                            "Evaluate each task against its Definition of Done.\n\n\
                             Task results:\n{results_summary}\n\n\
                             {files_list}\n\n\
                             {build_section}\n\n\
                             RULES:\n\
                             - If build FAILED: ALL tasks that modified files with errors are FAIL\n\
                             - If a task modified no files: FAIL (no deliverable produced)\n\
                             - Otherwise: PASS only if DoD criteria are met\n\n\
                             Do NOT use tools.\n\n\
                             Output format:\n\
                             ## Task: <task_id>\n\
                             - Status: **PASS** or **FAIL**\n\
                             - Reason: <one sentence>"
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

                TeamMessage::JrExecute {
                    task,
                    reply,
                    progress_tx,
                } => {
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

                    // Use the provided progress_tx if available, otherwise fall back to self.progress_tx
                    let tx = progress_tx.or_else(|| self.progress_tx.clone());

                    // Send sub-status update at the start of JrExecute
                    let sub_status = Self::extract_tool_info(&task.description);
                    send_progress(
                        &tx,
                        TeamProgressEvent::TaskSubStatusUpdate {
                            task_id: task_id.clone(),
                            message: format!("Starting: {}", sub_status),
                            nesting: 0,
                        },
                    );

                    let prompt_text = format!(
                        "Execute this task:\n{}\n\n\
                         Your task includes a DEFINITION_OF_DONE section. Complete only what the DoD requires — nothing more, nothing less.\
                         \n\n\
                         Do the work efficiently. Use the minimum number of tool calls needed (aim for 1-3). \
                         Do not verify or re-read files after writing them — trust the tool results. \
                         If CONTEXT is provided in the task, use it directly instead of reading the file. \
                         Do not run ls/cat/echo to verify your work.",
                        task.description
                    );

                    let task_result = match self.prompt(&prompt_text).await {
                        Ok(pr) => {
                            // Send completion sub-status update
                            let completion_msg = if pr.hit_tool_limit {
                                "Hit tool limit, retrying...".to_string()
                            } else if pr.files_modified.is_empty() {
                                "Task completed (no files modified)".to_string()
                            } else {
                                format!(
                                    "Completed ({} file{} modified)",
                                    pr.files_modified.len(),
                                    if pr.files_modified.len() == 1 {
                                        ""
                                    } else {
                                        "s"
                                    }
                                )
                            };
                            send_progress(
                                &tx,
                                TeamProgressEvent::TaskSubStatusUpdate {
                                    task_id: task_id.clone(),
                                    message: completion_msg,
                                    nesting: 0,
                                },
                            );

                            TaskResult {
                                task_id,
                                output: pr.text,
                                success: true,
                                hit_tool_limit: pr.hit_tool_limit,
                                files_modified: pr.files_modified,
                            }
                        }
                        Err(e) => {
                            // Send error sub-status update
                            send_progress(
                                &tx,
                                TeamProgressEvent::TaskSubStatusUpdate {
                                    task_id: task_id.clone(),
                                    message: format!("Failed: {}", e),
                                    nesting: 0,
                                },
                            );

                            TaskResult {
                                task_id,
                                output: format!("Task failed: {}", e),
                                success: false,
                                hit_tool_limit: false,
                                files_modified: vec![],
                            }
                        }
                    };

                    self.trim_history();
                    let _ = reply.send(task_result);
                    Ok(())
                }
            }
        })
    }
}
