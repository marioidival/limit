use crate::agent_bridge::{AgentBridge, AgentEvent};
use crate::error::CliError;
use crate::render::MarkdownRenderer;
use crate::session::SessionManager;
use rustyline::history::DefaultHistory;
use rustyline::{Config, Editor};
use std::io::Write;
use tokio::sync::mpsc;
use tracing::instrument;

pub struct Repl {
    reader: Editor<(), DefaultHistory>,
    session_manager: SessionManager,
    session_id: String,
    messages: Vec<limit_llm::Message>,
    agent_bridge: Option<AgentBridge>,
    event_rx: Option<mpsc::UnboundedReceiver<AgentEvent>>,
    total_input_tokens: u64,
    total_output_tokens: u64,
}

impl Repl {
    pub fn new() -> Result<Self, CliError> {
        let config = Config::builder().build();
        let reader = Editor::<(), DefaultHistory>::with_config(config)?;
        let session_manager = SessionManager::new()?;

        let session_id = match session_manager.get_last_session()? {
            Some(info) => {
                println!("Loading previous session: {}", info.id);
                info.id
            }
            None => {
                let new_id = session_manager.create_new_session()?;
                println!("Created new session: {}", new_id);
                new_id
            }
        };

        let messages = session_manager
            .load_session(&session_id)
            .unwrap_or_default();

        // Load config and initialize AgentBridge
        let config = limit_llm::Config::load()
            .map_err(|e| CliError::ConfigError(format!("Failed to load config: {}", e)))?;

        let (agent_bridge, event_rx) = if config
            .providers
            .get(&config.provider)
            .and_then(|p| p.api_key_or_env(&config.provider))
            .is_some()
        {
            let (tx, rx) = mpsc::unbounded_channel();
            let mut bridge = AgentBridge::new(config)?;
            bridge.set_event_tx(tx);
            println!(
                "Agent initialized with {} tools",
                bridge.get_tool_definitions().len()
            );
            (Some(bridge), Some(rx))
        } else {
            println!(
                "No API key found. Agent features disabled. Set api_key in ~/.limit/config.toml"
            );
            (None, None)
        };

        Ok(Self {
            reader,
            session_manager,
            session_id,
            messages,
            agent_bridge,
            event_rx,
            total_input_tokens: 0,
            total_output_tokens: 0,
        })
    }

    #[instrument(skip(self))]
    pub fn run(&mut self) -> Result<(), CliError> {
        println!("limit-cli - Interactive REPL");
        println!("Current session: {}", self.session_id);
        println!("Type /help for available commands\n");

        loop {
            let line = self.reader.readline("limit> ")?;

            self.process_line(line)?;
        }
    }

    #[instrument(skip(self, line))]
    fn process_line(&mut self, line: String) -> Result<(), CliError> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Ok(());
        }

        if let Some(cmd) = trimmed.strip_prefix("/") {
            self.handle_command(cmd)?;
        } else {
            self.handle_message(trimmed)?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn handle_command(&mut self, cmd: &str) -> Result<(), CliError> {
        match cmd {
            "exit" => {
                self.save_current_session()?;
                println!("Session saved. Goodbye!");
                std::process::exit(0);
            }
            "clear" => self.clear_screen()?,
            "help" => self.show_help(),
            "session list" => self.list_sessions()?,
            "session new" => self.new_session()?,
            "session load" => {
                eprintln!("Usage: /session load <session_id>");
            }
            _ if cmd.starts_with("session load ") => {
                let session_id = cmd.strip_prefix("session load ").unwrap();
                self.load_session(session_id)?;
            }
            "model" => {
                self.show_model_info();
            }
            _ => {
                println!(
                    "Unknown command: {}. Type /help for available commands.",
                    cmd
                );
            }
        }

        Ok(())
    }

    fn handle_message(&mut self, line: &str) -> Result<(), CliError> {
        self.reader.add_history_entry(line)?;

        // If agent_bridge is available, use it to process the message
        if let Some(ref mut bridge) = self.agent_bridge {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| CliError::IoError(std::io::Error::other(e)))?;

            let result =
                rt.block_on(async { bridge.process_message(line, &mut self.messages).await });

            // Process events from the agent
            if let Some(ref mut rx) = self.event_rx {
                let _renderer = MarkdownRenderer::new();
                while let Ok(event) = rx.try_recv() {
                    match event {
                        AgentEvent::Thinking => {
                            print!("\x1B[90mThinking...\x1B[0m");
                            std::io::stdout().flush()?;
                        }
                        AgentEvent::RequestStarted { turn, model } => {
                            print!("\x1B[90m\nTurn {}: {}\x1B[0m", turn, model);
                            std::io::stdout().flush()?;
                        }
                        AgentEvent::ReasoningChunk(reasoning) => {
                            // Reasoning is logged but not shown in REPL
                            tracing::debug!("Reasoning: {}", reasoning);
                        }
                        AgentEvent::ToolStart { name, args: _ } => {
                            println!("\x1B[90m\nTool: {}\x1B[0m", name);
                        }
                        AgentEvent::ToolComplete { name, result } => {
                            let _truncated = if result.len() > 500 {
                                format!("{}...", &result[..500])
                            } else {
                                result.clone()
                            };
                            println!("\x1B[90mResult: {} (truncated if needed)\x1B[0m", name);
                        }
                        AgentEvent::ContentChunk(_chunk) => {
                            // Don't stream content - will be shown in final response
                        }
                        AgentEvent::TokenUsage {
                            input_tokens,
                            output_tokens,
                        } => {
                            self.total_input_tokens += input_tokens;
                            self.total_output_tokens += output_tokens;
                            println!(
                                "\x1B[90mTokens: In: {} | Out: {}\x1B[0m",
                                input_tokens, output_tokens
                            );
                        }
                        AgentEvent::Done => {
                            println!();
                        }
                        AgentEvent::Error(err) => {
                            println!("\x1B[31mError: {}\x1B[0m", err);
                        }
                    }
                }
            }

            match result {
                Ok(response) => {
                    // Render the final response with markdown
                    if !response.is_empty() {
                        let renderer = MarkdownRenderer::new();
                        let rendered = renderer.render(&response);
                        println!("\n{}", rendered);
                    }
                }
                Err(e) => {
                    println!("\x1B[31mError processing message: {}\x1B[0m", e);
                    println!("Please check your API key and try again.");
                }
            }
        } else {
            // Fallback: just echo the message
            println!("You said: {}", line);
            println!("(Agent not configured. Add API key to ~/.limit/config.toml)");

            let user_message = limit_llm::Message {
                role: limit_llm::Role::User,
                content: Some(line.to_string()),
                tool_calls: None,
                tool_call_id: None,
            };

            self.messages.push(user_message);
        }

        Ok(())
    }

    fn clear_screen(&self) -> Result<(), CliError> {
        print!("\x1B[2J\x1B[1;1H");
        std::io::stdout().flush()?;
        Ok(())
    }

    fn show_help(&self) {
        println!("Available commands:");
        println!("  /help          - Show this help message");
        println!("  /clear         - Clear the screen");
        println!("  /exit          - Exit the REPL and save session");
        println!("  /session list  - List all sessions");
        println!("  /session new   - Create a new session");
        println!("  /session load  - Load a specific session by ID");
        println!("  /model         - Show current model configuration");
        println!("\nAny other input will be treated as a message and processed by the agent.");
    }

    fn show_model_info(&self) {
        if let Some(ref bridge) = self.agent_bridge {
            println!("Current model: {}", bridge.model());
            println!("Max tokens: {}", bridge.max_tokens());
            println!("Timeout: {} seconds", bridge.timeout());
            println!("Registered tools: {}", bridge.get_tool_definitions().len());
        } else {
            println!("Agent not configured. No model information available.");
        }
    }

    fn save_current_session(&self) -> Result<(), CliError> {
        self.session_manager.save_session(
            &self.session_id,
            &self.messages,
            self.total_input_tokens,
            self.total_output_tokens,
        )?;
        println!("Session {} saved.", self.session_id);
        Ok(())
    }

    fn list_sessions(&self) -> Result<(), CliError> {
        let sessions = self.session_manager.list_sessions()?;

        if sessions.is_empty() {
            println!("No sessions found.");
        } else {
            println!("Sessions (most recent first):\n");
            for (i, session) in sessions.iter().enumerate() {
                let current = if session.id == self.session_id {
                    " (current)"
                } else {
                    ""
                };
                println!(
                    "  {}. {}{} - {} messages",
                    i + 1,
                    &session.id[..8.min(session.id.len())],
                    current,
                    session.message_count
                );
            }
        }

        Ok(())
    }

    fn new_session(&mut self) -> Result<(), CliError> {
        self.save_current_session()?;

        let new_id = self.session_manager.create_new_session()?;
        self.session_id = new_id;
        self.messages.clear();
        self.total_input_tokens = 0;
        self.total_output_tokens = 0;

        println!("Created new session: {}", self.session_id);
        Ok(())
    }

    fn load_session(&mut self, session_id: &str) -> Result<(), CliError> {
        self.save_current_session()?;

        // Get session info including tokens from database
        let sessions = self.session_manager.list_sessions()?;
        let session_info = sessions
            .iter()
            .find(|s| s.id == session_id)
            .ok_or_else(|| CliError::ConfigError(format!("Session not found: {}", session_id)))?;

        let messages = self.session_manager.load_session(session_id)?;
        self.session_id = session_id.to_string();
        self.messages = messages;
        self.total_input_tokens = session_info.total_input_tokens;
        self.total_output_tokens = session_info.total_output_tokens;

        println!("Loaded session: {}", self.session_id);
        println!("Loaded {} messages.", self.messages.len());
        println!(
            "Total tokens: In: {} | Out: {}",
            self.total_input_tokens, self.total_output_tokens
        );
        Ok(())
    }
}
