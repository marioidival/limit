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
use futures::StreamExt;
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

    /// Stream a prompt, forwarding text chunks as progress events.
    /// Returns the full accumulated text.
    async fn stream_prompt(&mut self, input: &str) -> Result<String, AgentError> {
        let mut stream = std::pin::pin!(self.agent.prompt_stream(input));
        let mut full_text = String::new();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(text) => {
                    full_text.push_str(&text);
                    send_progress(
                        &self.progress_tx,
                        TeamProgressEvent::StreamChunk {
                            text: text.clone(),
                            nesting: 0,
                        },
                    );
                }
                Err(e) => return Err(e),
            }
        }
        Ok(full_text)
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
                        .stream_prompt(&format!(
                            "User request:\n{request}\n\n\
                             Analyze this request from a PRODUCT perspective.\n\
                             Do NOT explore code or file structure. Focus on user needs and business value.\n\n\
                             Structure your output as:\n\n\
                             ## Problem Statement\n<1-2 sentences describing the user's pain point>\n\n\
                             ## User Perspective\n- Who is affected?\n- What is their current workflow?\n- What friction do they experience?\n\n\
                             ## Business Value\n- Why does this matter?\n- What outcome does the user want?\n\n\
                             ## Requirements (Non-Technical)\n- <what the solution must accomplish, in user terms>\n- <focus on behavior and outcomes, not implementation>\n\n\
                             ## Success Criteria\n- <measurable outcome 1>\n- <measurable outcome 2>\n\n\
                             ## Ambiguities for TL to Resolve\n- <technical questions that need the Tech Lead's expertise>\n- <implementation details that aren't clear from the request>"
                        ))
                        .await;
                    self.log_event("analysis", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
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
                        .stream_prompt(&format!(
                            "Technical solution validated by TL:\n{validation}\n\n\
                             Task results:\n{results_summary}\n\n\
                             Files modified:\n{files_list}\n\n\
                             Summarize what was done: list the files created/modified and the outcome. \
                             Only suggest follow-up actions if something is genuinely incomplete or broken. \
                             Do NOT suggest improvements, refactors, or enhancements."
                        ))
                        .await;
                    self.log_event("delivery", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
                    Ok(())
                }

                TeamMessage::PmDeliverNoTasks { plan, reply, .. } => {
                    tracing::info!(
                        "[actor] PM received PmDeliverNoTasks ({} chars)",
                        plan.len()
                    );
                    let result = self
                        .stream_prompt(&format!(
                            "The Tech Lead produced a plan but no specific tasks. Here is the plan:\n\n{plan}\n\n\
                             Summarize this for the user and suggest next steps."
                        ))
                        .await;
                    self.log_event("delivery", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
                    Ok(())
                }

                TeamMessage::TlPlan { analysis, reply } => {
                    tracing::info!("[actor] TL received TlPlan ({} chars)", analysis.len());
                    let result = self
                        .stream_prompt(&format!(
                            "PM analysis:\n{analysis}\n\nCreate a detailed technical plan to implement this. \
                             Output the plan directly — do NOT say you will explore, read files, or investigate. \
                             You have no tools. Produce the plan now based on the analysis above."
                        ))
                        .await;
                    self.log_event("plan", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
                    Ok(())
                }

                TeamMessage::TlBreakdown { plan, reply } => {
                    tracing::info!("[actor] TL received TlBreakdown ({} chars)", plan.len());
                    let result = self
                        .stream_prompt(&format!(
                            "Technical plan:\n{plan}\n\n\
                             Break this down into at most {MAX_TASKS} specific, executable tasks.\n\
                             Each task must be self-contained and independently completable by a junior developer.\n\n\
                             IMPORTANT: Do NOT read any files or use tools. Junior agents have their own tools.\n\n\
                             ## Task Format (use EXACTLY this structure for each task):\n\n\
                             TASK: <clear description of what to implement>\n\
                             FILE_TARGETS: <exact file paths to create/modify, e.g., src/auth/login.rs>\n\
                             IMPLEMENTATION_HINTS:\n\
                             - <specific pattern to follow, e.g., 'Use the same error handling pattern as src/api/users.rs'>\n\
                             - <signature to implement, e.g., 'pub async fn login(email: &str, password: &str) -> Result<Token, Error>'>\n\
                             - <imports needed, e.g., 'use crate::models::User;'>\n\
                             - <algorithm/approach hint, e.g., 'Use bcrypt for password verification'>\n\
                             CONTEXT:\n\
                             ```<language>\n\
                             <relevant existing code: signatures, types, patterns to match>\n\
                             ```\n\
                             DEFINITION_OF_DONE:\n\
                             - <specific, verifiable criterion>\n\
                             - <e.g., 'Function compiles and is exported'>\n\
                             - <e.g., 'Returns correct type for valid input'>\n\
                             DEPENDS_ON: <task_id> (only if this task needs another task's output)\n\n\
                             ## Rules:\n\
                             - Every task MUST have FILE_TARGETS specifying exact paths\n\
                             - Every task MUST have IMPLEMENTATION_HINTS with specific guidance\n\
                             - CONTEXT blocks prevent Juniors from wasting tool calls exploring\n\
                             - Combine small steps into single tasks\n\
                             - Count every deliverable in the plan — never skip one\n\n\
                             Output ONLY the task list, starting with TASK: on each task."
                        ))
                        .await;
                    let _ = reply.send(result);
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
                        .stream_prompt(&format!(
                            "Files modified during task execution:\n{files_list}\n\n\
                             Based on the project structure, suggest a single shell command to verify \
                             that the changes compile/build correctly.\n\n\
                             Output ONLY the command, nothing else. No explanation, no markdown, no code blocks.\n\
                             If no build system is detected or files weren't modified, output: NONE"
                        ))
                        .await;
                    self.log_event("build_command", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
                    Ok(())
                }

                TeamMessage::TlValidate {
                    results_summary,
                    files_list,
                    build_cmd,
                    reply,
                } => {
                    tracing::info!(
                        "[actor] TL received TlValidate ({} chars, {} chars files list)",
                        results_summary.len(),
                        files_list.len()
                    );
                    let result = self
                        .stream_prompt(&format!(
                            "Evaluate each task against its Definition of Done.\n\n\
                             Task results:\n{results_summary}\n\n\
                             {files_list}\n\n\
                             BUILD COMMAND TO RUN: {build_cmd}\n\n\
                             Use your tools to verify:\n\
                             1. Run the build command above with bash\n\
                             2. Read relevant files with file_read to check DoD compliance\n\
                             3. Judge each task\n\n\
                             RULES:\n\
                             - If build fails: ALL tasks that modified files with errors are FAIL\n\
                             - If a task modified no files: FAIL (no deliverable produced)\n\
                             - Otherwise: PASS only if DoD criteria are met\n\n\
                             Output format:\n\
                             ## Task: <task_id>\n\
                             - Status: **PASS** or **FAIL**\n\
                             - Reason: <one sentence>"
                        ))
                        .await;
                    self.log_event("validation", &result.clone().unwrap_or_default())
                        .await;
                    let _ = reply.send(result);
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
                         INSTRUCTIONS:\n\
                         1. Identify FILE_TARGETS — create/modify exactly those files\n\
                         2. Follow IMPLEMENTATION_HINTS precisely — they specify patterns and signatures\n\
                         3. Use CONTEXT directly — do NOT re-read those files\n\
                         4. Complete only what DEFINITION_OF_DONE requires — nothing more\n\
                         5. Use minimum tool calls (1-3)\n\
                         6. Do NOT use bash for exploration (ls, find, cat, echo)\n\
                         7. Do NOT re-read files after writing\n\n\
                         Report what was done concisely when complete.",
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
