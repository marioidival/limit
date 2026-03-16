# ADR: Team Command Implementation

> **Status:** Proposed  
> **Date:** 2026-03-16  
> **Decision Makers:** Mário Idival  
> **ADR Number:** 001  

---

## Context and Problem Statement

### Current Situation

Limit is a Rust-based AI pair programmer CLI with:
- **17 built-in tools** (file I/O, bash, git, etc.)
- **Tool trait abstraction** (`limit-agent/src/tool.rs`)
- **Command system** (`limit-cli/src/tui/commands/`)
- **Multi-provider LLM support** (`limit-llm`)
- **Session persistence**
- **TUI + REPL modes**

### Problem

Users need to execute complex, multi-step tasks that require:
1. **Requirement analysis** (product vision)
2. **Technical planning** (architecture)
3. **Task breakdown** (specific implementation steps)
4. **Parallel execution** (multiple changes simultaneously)
5. **Validation** (testing, review)

Currently, a single agent handles all of this, which leads to:
- ❌ **Cognitive overload** (agent tries to do everything)
- ❌ **Sequential bottleneck** (no parallel execution)
- ❌ **No role specialization** (generic responses)
- ❌ **Harder debugging** (unclear which step failed)

### Proposed Solution

Implement a **`team` command** that creates a multi-agent system with specialized roles:

```
/team create --name "feature-team" --juniors 2
/team start --team "feature-team" --task "Add JWT authentication"
```

**Roles:**
- **PM** (Product Manager): Understands requirements, product vision
- **TL** (Tech Lead): Architecture, task breakdown, validation
- **Jr** (Junior Developer): Executes tasks with tools

---

## Decision

### ADR-001: Implement Multi-Agent Team System

**Status:** ✅ Approved

**Decision:** Implement a `team` command using **Limit's existing architecture** (Tool trait, ToolRegistry, LLM providers) **WITHOUT external dependencies** (no Rig, LangChain, etc.).

**Rationale:**
1. ✅ **No new dependencies** (keep Limit lightweight)
2. ✅ **Consistent patterns** (use existing Tool trait)
3. ✅ **Full control** (own implementation, easier debugging)
4. ✅ **Lower learning curve** (team members already know Limit's code)
5. ✅ **Performance** (no framework overhead)

---

## Architecture Design

### 1. **Component Structure**

```
limit/
├── limit-cli/
│   └── src/
│       └── tui/
│           └── commands/
│               └── team.rs         # NEW: Team command
├── limit-agent/
│   └── src/
│       ├── team/
│       │   ├── mod.rs              # NEW: Team module
│       │   ├── agent.rs            # NEW: Specialized agent
│       │   ├── role.rs             # NEW: PM/TL/Jr roles
│       │   ├── workflow.rs         # NEW: Team workflow
│       │   ├── orchestrator.rs     # NEW: Task orchestration
│       │   └── history.rs          # NEW: Team history
│       ├── tool.rs                 # EXISTING: Tool trait
│       ├── registry.rs             # EXISTING: ToolRegistry
│       └── lib.rs                  # UPDATE: Export team
└── limit-llm/
    └── src/
        ├── providers/              # EXISTING: LLM providers
        └── message.rs              # EXISTING: Message types
```

---

### 2. **Core Abstractions**

#### **2.1 Role Enum**

```rust
// limit-agent/src/team/role.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    /// Product Manager - understands requirements
    PM,
    
    /// Tech Lead - architecture and validation
    TL,
    
    /// Junior Developer - executes tasks
    Jr,
}

impl Role {
    pub fn system_prompt(&self) -> &str {
        match self {
            Role::PM => include_str!("prompts/pm.md"),
            Role::TL => include_str!("prompts/tl.md"),
            Role::Jr => include_str!("prompts/jr.md"),
        }
    }
    
    pub fn default_model(&self) -> &str {
        match self {
            Role::PM => "gpt-4",
            Role::TL => "gpt-4",
            Role::Jr => "gpt-4o-mini", // Cheaper for execution
        }
    }
    
    pub fn available_tools(&self) -> Vec<&str> {
        match self {
            Role::PM => vec![], // No tools, just reasoning
            Role::TL => vec!["bash"], // Can test/validate
            Role::Jr => vec!["file_read", "file_write", "file_edit", "bash"],
        }
    }
}
```

---

#### **2.2 TeamAgent Trait**

```rust
// limit-agent/src/team/agent.rs

use crate::tool::Tool;
use crate::error::AgentError;
use limit_llm::{Message, Provider};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// A specialized agent with a specific role
pub struct TeamAgent {
    role: Role,
    provider: Arc<dyn Provider>,
    model: String,
    tools: Vec<Arc<dyn Tool>>,
    history: Vec<Message>,
}

impl TeamAgent {
    pub fn new(
        role: Role,
        provider: Arc<dyn Provider>,
        model: Option<String>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            role,
            provider,
            model: model.unwrap_or_else(|| role.default_model().to_string()),
            tools,
            history: vec![Message::system(role.system_prompt())],
        }
    }
    
    pub async fn prompt(&mut self, user_input: &str) -> Result<String, AgentError> {
        self.history.push(Message::user(user_input));
        
        let response = self.provider
            .complete(&self.model, &self.history, &self.tools)
            .await?;
        
        // Handle tool calls if present
        let final_response = self.handle_tool_calls(response).await?;
        
        self.history.push(Message::assistant(&final_response));
        
        Ok(final_response)
    }
    
    pub async fn prompt_stream(
        &mut self,
        user_input: &str,
    ) -> Result<impl futures::Stream<Item = String>, AgentError> {
        self.history.push(Message::user(user_input));
        
        let stream = self.provider
            .complete_stream(&self.model, &self.history, &self.tools)
            .await?;
        
        Ok(stream)
    }
    
    async fn handle_tool_calls(&mut self, response: String) -> Result<String, AgentError> {
        // Parse tool calls from response
        let tool_calls = self.parse_tool_calls(&response)?;
        
        if tool_calls.is_empty() {
            return Ok(response);
        }
        
        // Execute each tool call
        let mut results = vec![];
        for (tool_name, args) in tool_calls {
            let tool = self.tools.iter()
                .find(|t| t.name() == tool_name)
                .ok_or_else(|| AgentError::ToolNotFound(tool_name.clone()))?;
            
            let result = tool.execute(args).await?;
            results.push((tool_name, result));
        }
        
        // Continue conversation with tool results
        let tool_results_message = format!(
            "Tool results:\n{}",
            results.iter()
                .map(|(name, result)| format!("- {}: {}", name, result))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        self.history.push(Message::user(&tool_results_message));
        
        let final_response = self.provider
            .complete(&self.model, &self.history, &self.tools)
            .await?;
        
        Ok(final_response)
    }
    
    fn parse_tool_calls(&self, response: &str) -> Result<Vec<(String, Value)>, AgentError> {
        // Look for tool call patterns in response
        // Example: [TOOL_CALL: file_write {"path": "test.rs", "content": "..."}]
        let mut calls = vec![];
        
        let re = regex::Regex::new(r"\[TOOL_CALL:\s*(\w+)\s+(\{.*?\})\]")
            .map_err(|e| AgentError::ParseError(e.to_string()))?;
        
        for cap in re.captures_iter(response) {
            let tool_name = cap[1].to_string();
            let args: Value = serde_json::from_str(&cap[2])
                .map_err(|e| AgentError::ParseError(e.to_string()))?;
            calls.push((tool_name, args));
        }
        
        Ok(calls)
    }
    
    pub fn role(&self) -> &Role {
        &self.role
    }
    
    pub fn history(&self) -> &[Message] {
        &self.history
    }
    
    pub fn clear_history(&mut self) {
        self.history = vec![Message::system(self.role.system_prompt())];
    }
}
```

---

#### **2.3 Team Struct**

```rust
// limit-agent/src/team/mod.rs

mod agent;
mod role;
mod workflow;
mod orchestrator;
mod history;

pub use agent::TeamAgent;
pub use role::Role;
pub use workflow::TeamWorkflow;
pub use orchestrator::TaskOrchestrator;
pub use history::TeamHistory;

use crate::error::AgentError;
use crate::tool::Tool;
use limit_llm::Provider;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Team {
    pub name: String,
    pub pm: TeamAgent,
    pub tl: TeamAgent,
    pub jrs: Vec<TeamAgent>,
    pub history: Arc<RwLock<TeamHistory>>,
    pub config: TeamConfig,
}

#[derive(Debug, Clone)]
pub struct TeamConfig {
    pub pm_model: Option<String>,
    pub tl_model: Option<String>,
    pub jr_model: Option<String>,
    pub num_juniors: usize,
    pub max_parallel_tasks: usize,
    pub enable_streaming: bool,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            pm_model: None,
            tl_model: None,
            jr_model: None,
            num_juniors: 2,
            max_parallel_tasks: 4,
            enable_streaming: true,
        }
    }
}

impl Team {
    pub async fn new(
        name: String,
        provider: Arc<dyn Provider>,
        config: TeamConfig,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Result<Self, AgentError> {
        // Create PM agent
        let pm_tools = filter_tools(&tools, Role::PM.available_tools());
        let pm = TeamAgent::new(
            Role::PM,
            provider.clone(),
            config.pm_model.clone(),
            pm_tools,
        );
        
        // Create TL agent
        let tl_tools = filter_tools(&tools, Role::TL.available_tools());
        let tl = TeamAgent::new(
            Role::TL,
            provider.clone(),
            config.tl_model.clone(),
            tl_tools,
        );
        
        // Create Jr agents
        let jr_tools = filter_tools(&tools, Role::Jr.available_tools());
        let jrs = (0..config.num_juniors)
            .map(|_| {
                TeamAgent::new(
                    Role::Jr,
                    provider.clone(),
                    config.jr_model.clone(),
                    jr_tools.clone(),
                )
            })
            .collect();
        
        Ok(Self {
            name,
            pm,
            tl,
            jrs,
            history: Arc::new(RwLock::new(TeamHistory::new())),
            config,
        })
    }
    
    pub async fn execute(&mut self, user_request: &str) -> Result<TeamResult, AgentError> {
        let start = std::time::Instant::now();
        
        // Step 1: PM analyzes request
        log_team_event(&self.name, "PM", "Analyzing request");
        let analysis = self.pm.prompt(&format!(
            "User request: {}\n\nAnalyze this request and identify what needs to be done.",
            user_request
        )).await?;
        
        self.log_event("PM", "analysis", &analysis).await;
        
        // Step 2: PM + TL discuss
        log_team_event(&self.name, "TL", "Receiving requirements");
        let plan = self.tl.prompt(&format!(
            "PM analysis: {}\n\nCreate a technical plan to implement this.",
            analysis
        )).await?;
        
        self.log_event("TL", "plan", &plan).await;
        
        // Step 3: TL breaks down tasks
        log_team_event(&self.name, "TL", "Breaking down tasks");
        let tasks = self.breakdown_tasks(&plan).await?;
        
        self.log_event("TL", "tasks", &serde_json::to_string(&tasks)?).await;
        
        // Step 4: Execute tasks in parallel
        log_team_event(&self.name, "Jr", "Executing tasks");
        let results = self.execute_tasks_parallel(tasks).await?;
        
        // Step 5: TL validates
        log_team_event(&self.name, "TL", "Validating results");
        let validation = self.tl.prompt(&format!(
            "Task results:\n{}\n\nValidate the implementation and ensure quality.",
            serde_json::to_string_pretty(&results)?
        )).await?;
        
        self.log_event("TL", "validation", &validation).await;
        
        // Step 6: PM delivers
        log_team_event(&self.name, "PM", "Delivering solution");
        let delivery = self.pm.prompt(&format!(
            "Technical solution:\n{}\n\nSummarize the solution for the user.",
            validation
        )).await?;
        
        self.log_event("PM", "delivery", &delivery).await;
        
        let duration = start.elapsed();
        
        let files_modified = self.extract_modified_files().await;
        
        Ok(TeamResult {
            solution: delivery,
            duration,
            files_modified,
            history: self.history.read().await.clone(),
        })
    }
    
    async fn breakdown_tasks(&mut self, plan: &str) -> Result<Vec<Task>, AgentError> {
        let tasks_response = self.tl.prompt(&format!(
            "Technical plan: {}\n\nBreak this down into specific, executable tasks. Format each task as:\nTASK: <description>",
            plan
        )).await?;
        
        // Parse tasks from response
        let tasks = tasks_response
            .lines()
            .filter(|line| line.starts_with("TASK:"))
            .map(|line| Task {
                id: uuid::Uuid::new_v4().to_string(),
                description: line.trim_start_matches("TASK:").trim().to_string(),
                status: TaskStatus::Pending,
            })
            .collect();
        
        Ok(tasks)
    }
    
    async fn execute_tasks_parallel(&mut self, tasks: Vec<Task>) -> Result<Vec<TaskResult>, AgentError> {
        use futures::stream::{self, StreamExt};
        
        let results: Vec<Result<TaskResult, AgentError>> = stream::iter(tasks)
            .enumerate()
            .map(|(i, task)| {
                let jr_idx = i % self.jrs.len();
                let jr = &mut self.jrs[jr_idx];
                
                async move {
                    let result = jr.prompt(&format!(
                        "Execute this task:\n{}\n\nUse tools as needed.",
                        task.description
                    )).await?;
                    
                    Ok(TaskResult {
                        task_id: task.id,
                        output: result,
                        success: true,
                    })
                }
            })
            .buffer_unordered(self.config.max_parallel_tasks)
            .collect()
            .await;
        
        // Collect successful results
        let mut successful = vec![];
        for result in results {
            match result {
                Ok(task_result) => successful.push(task_result),
                Err(e) => log::error!("Task failed: {}", e),
            }
        }
        
        Ok(successful)
    }
    
    async fn log_event(&self, role: &str, action: &str, content: &str) {
        let mut history = self.history.write().await;
        history.add_event(TeamEvent {
            timestamp: chrono::Utc::now(),
            role: role.to_string(),
            action: action.to_string(),
            content: content.to_string(),
        });
    }
    
    async fn extract_modified_files(&self) -> Vec<String> {
        // Parse file paths from Jr agent tool calls
        let history = self.history.read().await;
        let mut files = vec![];
        
        for event in history.events() {
            if event.role == "Jr" && event.action.contains("file") {
                // Extract file paths from tool calls
                if let Some(path) = extract_file_path(&event.content) {
                    files.push(path);
                }
            }
        }
        
        files.sort();
        files.dedup();
        files
    }
}

fn filter_tools(tools: &[Arc<dyn Tool>], allowed: Vec<&str>) -> Vec<Arc<dyn Tool>> {
    tools.iter()
        .filter(|tool| allowed.contains(&tool.name()))
        .cloned()
        .collect()
}

fn log_team_event(team_name: &str, role: &str, action: &str) {
    tracing::info!("[Team:{}] {} - {}", team_name, role, action);
}

fn extract_file_path(content: &str) -> Option<String> {
    // Extract file path from tool call
    let re = regex::Regex::new(r#""path":\s*"([^"]+)""#).ok()?;
    let cap = re.captures(content)?;
    Some(cap[1].to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResult {
    pub solution: String,
    pub duration: std::time::Duration,
    pub files_modified: Vec<String>,
    pub history: TeamHistory,
}
```

---

### 3. **Command Implementation**

```rust
// limit-cli/src/tui/commands/team.rs

use crate::tui::commands::{Command, CommandContext, CommandResult};
use crate::tui::state::AppState;
use limit_agent::team::{Team, TeamConfig};
use limit_llm::Provider;
use std::sync::Arc;

pub struct TeamCommand {
    teams: Arc<tokio::sync::RwLock<Vec<Team>>>,
}

impl TeamCommand {
    pub fn new() -> Self {
        Self {
            teams: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

impl Command for TeamCommand {
    fn name(&self) -> &str {
        "team"
    }
    
    fn description(&self) -> &str {
        "Manage and execute tasks with multi-agent teams"
    }
    
    fn usage(&self) -> &str {
        r#"
/team <subcommand> [options]

Subcommands:
  create   Create a new team
  delete   Delete a team
  list     List all teams
  status   Show team status
  start    Start a team task
  history  Show team history

Examples:
  /team create --name "dev-team" --juniors 2
  /team list
  /team start --team "dev-team" --task "Add JWT authentication"
  /team status "dev-team"
  /team delete "dev-team"
"#
    }
    
    async fn execute(
        &self,
        args: &str,
        context: &mut CommandContext,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(CommandResult::Error(
                "Usage: /team <subcommand>".to_string()
            ));
        }
        
        let subcommand = parts[0];
        
        match subcommand {
            "create" => self.handle_create(&parts[1..], context).await,
            "delete" => self.handle_delete(&parts[1..]).await,
            "list" => self.handle_list().await,
            "status" => self.handle_status(&parts[1..]).await,
            "start" => self.handle_start(&parts[1..], context).await,
            "history" => self.handle_history(&parts[1..]).await,
            _ => Ok(CommandResult::Error(format!(
                "Unknown subcommand: {}",
                subcommand
            ))),
        }
    }
}

impl TeamCommand {
    async fn handle_create(
        &self,
        args: &[&str],
        context: &CommandContext,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let mut name = None;
        let mut juniors = 2;
        
        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--name" | "-n" => {
                    name = args.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "--juniors" | "-j" => {
                    juniors = args.get(i + 1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(2);
                    i += 2;
                }
                _ => i += 1,
            }
        }
        
        let name = name.ok_or("Team name required. Use --name <name>")?;
        
        // Create team
        let provider = context.provider.clone();
        let tools = context.tools.clone();
        
        let config = TeamConfig {
            num_juniors: juniors,
            ..Default::default()
        };
        
        let team = Team::new(name.clone(), provider, config, tools).await?;
        
        // Store team
        self.teams.write().await.push(team);
        
        Ok(CommandResult::Success(format!(
            "✅ Team '{}' created with {} junior agents",
            name, juniors
        )))
    }
    
    async fn handle_start(
        &self,
        args: &[&str],
        context: &mut CommandContext,
    ) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let mut team_name = None;
        let mut task = None;
        
        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--team" | "-t" => {
                    team_name = args.get(i + 1).map(|s| s.to_string());
                    i += 2;
                }
                "--task" | "-k" => {
                    // Collect remaining args as task
                    task = Some(args[i + 1..].join(" "));
                    break;
                }
                _ => i += 1,
            }
        }
        
        let team_name = team_name.ok_or("Team name required. Use --team <name>")?;
        let task = task.ok_or("Task required. Use --task <description>")?;
        
        // Find team
        let teams = self.teams.read().await;
        let team_idx = teams.iter()
            .position(|t| t.name == team_name)
            .ok_or_else(|| format!("Team '{}' not found", team_name))?;
        
        drop(teams);
        
        // Execute task
        let mut teams = self.teams.write().await;
        let team = &mut teams[team_idx];
        
        let result = team.execute(&task).await?;
        
        Ok(CommandResult::Success(format!(
            "✅ Task completed in {:?}\n\n{}\n\nFiles modified:\n{}",
            result.duration,
            result.solution,
            result.files_modified.join("\n")
        )))
    }
    
    async fn handle_list(&self) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let teams = self.teams.read().await;
        
        if teams.is_empty() {
            return Ok(CommandResult::Success("No teams created yet.".to_string()));
        }
        
        let list = teams.iter()
            .map(|t| format!("- {} ({} juniors)", t.name, t.jrs.len()))
            .collect::<Vec<_>>()
            .join("\n");
        
        Ok(CommandResult::Success(format!("Teams:\n{}", list)))
    }
    
    async fn handle_delete(&self, args: &[&str]) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let name = args.first().ok_or("Team name required")?;
        
        let mut teams = self.teams.write().await;
        let idx = teams.iter()
            .position(|t| t.name == *name)
            .ok_or_else(|| format!("Team '{}' not found", name))?;
        
        teams.remove(idx);
        
        Ok(CommandResult::Success(format!("✅ Team '{}' deleted", name)))
    }
    
    async fn handle_status(&self, args: &[&str]) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let name = args.first().ok_or("Team name required")?;
        
        let teams = self.teams.read().await;
        let team = teams.iter()
            .find(|t| t.name == *name)
            .ok_or_else(|| format!("Team '{}' not found", name))?;
        
        let status = format!(
            "Team: {}\nPM: ready\nTL: ready\nJuniors: {} agents",
            team.name,
            team.jrs.len()
        );
        
        Ok(CommandResult::Success(status))
    }
    
    async fn handle_history(&self, args: &[&str]) -> Result<CommandResult, Box<dyn std::error::Error>> {
        let name = args.first().ok_or("Team name required")?;
        
        let teams = self.teams.read().await;
        let team = teams.iter()
            .find(|t| t.name == *name)
            .ok_or_else(|| format!("Team '{}' not found", name))?;
        
        let history = team.history.read().await;
        
        let events = history.events()
            .iter()
            .map(|e| format!("[{}] {}: {}", e.role, e.action, e.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        
        Ok(CommandResult::Success(events))
    }
}
```

---

### 4. **Register Command**

```rust
// limit-cli/src/tui/commands/mod.rs

mod team;

pub use team::TeamCommand;

pub fn create_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(ClearCommand));
    registry.register(Box::new(ExitCommand));
    registry.register(Box::new(SessionCommand::new()));
    registry.register(Box::new(ShareCommand::new()));
    registry.register(Box::new(TeamCommand::new())); // NEW

    registry
}
```

---

### 5. **Prompt Templates**

```markdown
<!-- limit-agent/src/team/prompts/pm.md -->

You are a Product Manager in a multi-agent development team.

Your responsibilities:
- Understand user requirements from a product perspective
- Identify business value and user needs
- Clarify ambiguous requirements
- Ensure solutions meet product goals
- Communicate clearly with the Tech Lead

Guidelines:
- Be concise but thorough
- Ask clarifying questions when needed
- Focus on WHAT needs to be done, not HOW
- Consider edge cases and user experience
- Think about scalability and maintainability

When analyzing a request:
1. Identify the core problem
2. List key requirements
3. Note any ambiguities
4. Suggest clarifications if needed
5. Define success criteria

Example output:
"""
Request Analysis:
- Core problem: User needs to authenticate before accessing protected routes
- Key requirements: Login form, JWT tokens, session management
- Ambiguities: OAuth support? 2FA? Password reset?
- Success criteria: User can log in and access protected routes
"""
```

```markdown
<!-- limit-agent/src/team/prompts/tl.md -->

You are a Tech Lead in a multi-agent development team.

Your responsibilities:
- Design technical architecture
- Break down requirements into specific tasks
- Delegate tasks to Junior developers
- Review and validate implementations
- Ensure code quality and best practices

Guidelines:
- Be precise and technical
- Break down complex tasks into smaller steps
- Provide clear instructions for Juniors
- Consider testing and error handling
- Think about performance and security

When creating a technical plan:
1. Identify components to create/modify
2. Define data structures and APIs
3. Break down into specific tasks
4. Specify file paths and naming
5. Define validation criteria

When breaking down tasks:
Format each task as:
TASK: <clear, specific instruction>

Example:
TASK: Create src/auth/jwt.ts with JWT generation and validation functions
TASK: Create src/middleware/auth.ts with authentication middleware
TASK: Add /auth/login endpoint in src/routes/auth.ts

When using tools, format as:
[TOOL_CALL: bash {"command": "npm test"}]
"""
```

```markdown
<!-- limit-agent/src/team/prompts/jr.md -->

You are a Junior Developer in a multi-agent development team.

Your responsibilities:
- Execute specific tasks assigned by the Tech Lead
- Write clean, working code
- Follow coding standards
- Test your implementations
- Report back with results or questions

Guidelines:
- Focus on one task at a time
- Use tools to read, write, and edit files
- Follow existing code patterns
- Add error handling
- Test before reporting completion

Available tools:
- file_read: Read file contents
- file_write: Create new files
- file_edit: Edit existing files
- bash: Run shell commands

When using tools, format as:
[TOOL_CALL: file_write {"path": "src/file.ts", "content": "..."}]

After completing a task:
1. Verify the implementation works
2. Report what was done
3. Note any issues or questions

Example output:
"""
[TOOL_CALL: file_write {"path": "src/auth/jwt.ts", "content": "export function generateToken(userId: string): string { ... }"}]

✅ Created src/auth/jwt.ts with generateToken and validateToken functions.
"""
```

---

## Implementation Roadmap

### Phase 1: Core Infrastructure (Week 1)

**Goal:** Basic team creation and task execution

**Tasks:**
- [ ] Create `limit-agent/src/team/` module
- [ ] Implement `Role` enum
- [ ] Implement `TeamAgent` struct
- [ ] Implement `Team` struct
- [ ] Add prompt templates
- [ ] Write unit tests

**Deliverables:**
```rust
let team = Team::new("dev-team", provider, config, tools).await?;
let result = team.execute("Add hello world function").await?;
```

---

### Phase 2: Command Integration (Week 2)

**Goal:** CLI command for team management

**Tasks:**
- [ ] Implement `TeamCommand` in `limit-cli`
- [ ] Add subcommands: create, delete, list, status, start
- [ ] Integrate with existing `CommandRegistry`
- [ ] Add command help text
- [ ] Test CLI usage

**Deliverables:**
```bash
lim> /team create --name "dev-team"
lim> /team start --team "dev-team" --task "Add JWT auth"
```

---

### Phase 3: Advanced Features (Week 3)

**Goal:** Parallel execution and history

**Tasks:**
- [ ] Implement parallel task execution
- [ ] Add `TeamHistory` for event logging
- [ ] Implement streaming output
- [ ] Add error recovery
- [ ] Performance optimization

**Deliverables:**
```rust
// Parallel execution
let results = team.execute_tasks_parallel(tasks).await?;

// History tracking
let history = team.history.read().await;
```

---

### Phase 4: Polish & Testing (Week 4)

**Goal:** Production-ready implementation

**Tasks:**
- [ ] Add configuration options
- [ ] Improve error messages
- [ ] Add integration tests
- [ ] Update documentation
- [ ] Add examples to README

**Deliverables:**
- Full test coverage (>80%)
- User documentation
- Example workflows

---

## Configuration

```toml
# ~/.limit/config.toml

[team]
# Default number of junior agents
default_juniors = 2

# Maximum parallel tasks
max_parallel_tasks = 4

# Enable streaming output
streaming = true

# Model configurations
[team.models]
pm = "gpt-4"
tl = "gpt-4"
jr = "gpt-4o-mini"

# Team storage
teams_dir = "~/.limit/teams"
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_team_creation() {
        let provider = create_mock_provider();
        let tools = vec![];
        let config = TeamConfig::default();
        
        let team = Team::new("test-team", provider, config, tools).await.unwrap();
        
        assert_eq!(team.name, "test-team");
        assert_eq!(team.jrs.len(), 2);
    }
    
    #[tokio::test]
    async fn test_task_breakdown() {
        let mut team = create_test_team().await;
        
        let tasks = team.breakdown_tasks("Implement user authentication").await.unwrap();
        
        assert!(!tasks.is_empty());
        assert!(tasks[0].description.contains("TASK:"));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_team_workflow() {
    let team = create_real_team().await;
    
    let result = team.execute("Create a simple hello world function").await.unwrap();
    
    assert!(!result.solution.is_empty());
    assert!(result.duration.as_secs() > 0);
}
```

---

## Performance Considerations

### 1. **Provider Calls**

| Operation | Calls | Optimization |
|-----------|-------|--------------|
| PM analysis | 1 | - |
| TL plan | 1 | - |
| TL breakdown | 1 | - |
| Jr execution | N (parallel) | Parallel execution |
| TL validation | 1 | - |
| PM delivery | 1 | - |
| **Total** | **5 + N** | N in parallel |

### 2. **Memory**

- **Team history**: ~100KB per team (configurable retention)
- **Agent context**: ~10KB per agent
- **Total**: ~150KB per team

### 3. **Latency**

| Phase | Latency | Parallel? |
|-------|---------|-----------|
| PM analysis | 2-5s | No |
| TL plan | 2-5s | No |
| TL breakdown | 1-3s | No |
| Jr execution | 5-20s total | **Yes** |
| TL validation | 2-5s | No |
| PM delivery | 1-3s | No |
| **Total** | **15-40s** | - |

---

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **LLM rate limits** | High | Medium | Exponential backoff, queue tasks |
| **Tool execution failures** | Medium | Medium | Retry logic, error handling |
| **Parallel execution race conditions** | Medium | Low | File locking, sequential fallback |
| **Memory leaks (long-running teams)** | Low | Low | Clear history periodically |
| **Conflicting task assignments** | Medium | Medium | TL coordinates, Jr reports conflicts |

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Task completion rate** | >95% | Successful task executions |
| **Time to completion** | <60s for simple tasks | Average execution time |
| **User satisfaction** | >4.5/5 | User feedback |
| **Error rate** | <5% | Failed executions |
| **Parallelization efficiency** | >80% | Speedup vs sequential |

---

## Future Enhancements

### Phase 5: Advanced Orchestration

- [ ] Dynamic role assignment
- [ ] Task dependencies (DAG execution)
- [ ] Agent communication protocols
- [ ] Learning from past executions

### Phase 6: Team Templates

- [ ] Pre-configured team types (frontend, backend, devops)
- [ ] Custom role definitions
- [ ] Tool whitelists per role

### Phase 7: Collaboration

- [ ] Multiple users per team
- [ ] Team sharing
- [ ] Remote team execution

---

## References

- **Limit Repository:** https://github.com/marioidival/limit
- **Tool Trait:** `limit-agent/src/tool.rs`
- **Command System:** `limit-cli/src/tui/commands/`
- **LLM Providers:** `limit-llm/src/providers/`

---

## Approval

| Role | Name | Date | Status |
|------|------|------|--------|
| Author | OpenClaw | 2026-03-16 | Draft |
| Reviewer | Mário Idival | - | Pending |
| Approver | Mário Idival | - | Pending |

---

## Appendix: Full Code Example

```rust
// Example usage in Limit CLI

lim> /team create --name "auth-team" --juniors 3
✅ Team 'auth-team' created with 3 junior agents

lim> /team start --team "auth-team" --task "Implement user authentication with JWT tokens"

💬 PM: Analyzing request...
   "Request Analysis:
   - Core problem: User authentication for protected routes
   - Requirements: Login form, JWT generation, middleware
   - Ambiguities: Password reset? OAuth? 2FA?
   - Success: User can log in and access protected routes"

💬 TL: Creating technical plan...
   "Plan:
   1. Create JWT utility functions
   2. Implement authentication middleware
   3. Add login endpoint
   4. Create login form component
   5. Add protected route example"

💬 TL: Breaking down tasks...
   "TASK: Create src/utils/jwt.ts with sign and verify functions
   TASK: Create src/middleware/auth.ts with requireAuth middleware
   TASK: Add POST /auth/login endpoint in src/routes/auth.ts
   TASK: Create src/components/LoginForm.tsx
   TASK: Add protected route example in src/App.tsx"

💬 Jr[0]: Executing task 1...
   [TOOL_CALL: file_write {...}]
   ✅ Created src/utils/jwt.ts

💬 Jr[1]: Executing task 2...
   [TOOL_CALL: file_write {...}]
   ✅ Created src/middleware/auth.ts

💬 Jr[2]: Executing task 3...
   [TOOL_CALL: file_write {...}]
   ✅ Created src/routes/auth.ts

💬 Jr[0]: Executing task 4...
   [TOOL_CALL: file_write {...}]
   ✅ Created src/components/LoginForm.tsx

💬 Jr[1]: Executing task 5...
   [TOOL_CALL: file_edit {...}]
   ✅ Updated src/App.tsx

💬 TL: Validating results...
   ✅ All tasks completed successfully
   ✅ Code follows conventions
   ⚠️ Minor: Add error handling to LoginForm

💬 PM: Delivering solution...
   "✅ JWT Authentication implemented!
   
   Files created:
   - src/utils/jwt.ts
   - src/middleware/auth.ts
   - src/routes/auth.ts
   - src/components/LoginForm.tsx
   - src/App.tsx (updated)
   
   Duration: 28.4s
   
   Next steps: Test login flow, add password reset feature"

✅ Task completed in 28.4s

lim> /team history "auth-team"

[PM] analysis: Request Analysis...
[TL] plan: Plan: 1. Create JWT...
[TL] tasks: TASK: Create src/utils/jwt.ts...
[Jr] task 1: Created src/utils/jwt.ts
[Jr] task 2: Created src/middleware/auth.ts
...
```

---

**End of ADR**
