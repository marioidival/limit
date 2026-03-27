# Input Queue System

## Overview

Limit implements a two-tier input queue system based on Codex's design, allowing users to continue interacting while async operations run.

## Architecture

```
┌─────────────────────────────────────────┐
│            Input Flow                    │
└─────────────────────────────────────────┘

User types message
       │
       ▼
  ┌────────────┐
  │ Task       │──── Yes ────→ queued_messages.push_back()
  │ Running?   │                    │
  └────────────┘                    │ refresh_preview()
       │                            │
       No                           │
       ▼                            ▼
  submit_message()           Preview Widget
       │                     shows message
       ▼                            │
  pending_steers.push()             │
       │                            │ Task ends
       ▼                            ▼
  Wait for commit           maybe_send_next_queued()
       │                            │
       ▼                            ▼
  Committed ──────────> pop_front() → submit_message()
```

## Implementations

### limit-tui (`InputQueueManager`)

Full-featured queue manager with state machine:

```rust
use limit_tui::components::{InputQueueManager, UserMessage, InputState};

let mut queue = InputQueueManager::new();

// Start a task
queue.on_task_started(true);

// Queue message while running
queue.submit_message(UserMessage::new("Continue".into()), |_| Ok(()))?;

// Preview pending
let (queued, pending) = queue.preview_data();

// Task completes - auto-send next
queue.on_task_completed(|msg| { /* send */ })?;
```

### limit-cli (`InputQueue`)

Simplified wrapper for CLI usage:

```rust
use limit_cli::tui::input_queue::{InputQueue, QueueConfig};

// With default config
let mut queue = InputQueue::new();

// With custom config
let config = QueueConfig {
    max_queued_messages: 100,
    max_pending_steers: 20,
};
let mut queue = InputQueue::with_config(config);

// Basic operations
queue.queue_message("Hello".to_string());
queue.add_steer("Continue".to_string());

let merged = queue.merge_all(); // Combines all into one message
```

## State Machine

```
Idle ──(task starts)──> TaskRunning
TaskRunning ──(task completes)──> Idle
TaskRunning ──(ESC pressed)──> Interrupting
Interrupting ──(interrupt ack)──> TaskRunning or Idle
```

### States

| State | Description |
|-------|-------------|
| `Idle` | No task running, input sends immediately |
| `TaskRunning` | Task executing, input queues |
| `Interrupting` | Interrupt sent, awaiting acknowledgment |

## Key Bindings

| Key | Action |
|-----|--------|
| `Enter` | Submit message (or queue if task running) |
| `ESC` | Request interrupt (send pending steers) |
| `ESC ESC` | Force cancel (after 500ms timeout) |
| `Alt+↑` | Edit last queued message |

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `max_queued_messages` | 50 | Queue capacity before eviction |
| `max_pending_steers` | 10 | Max steers before blocking |
| `interrupt_timeout_ms` | 500 | Double-ESC window |
| `preview_line_limit` | 3 | Lines per message in preview |

### Custom Configuration

```rust
use limit_cli::tui::input_queue::{InputQueue, QueueConfig};

let config = QueueConfig {
    max_queued_messages: 100,
    max_pending_steers: 15,
};

let mut queue = InputQueue::with_config(config);
```

## Thread State Persistence

Switch between conversation threads while preserving queue state:

```rust
// Save current thread state
let state = queue.save_thread_state();

// Switch to different thread
queue.clear();
queue.queue_message("Other thread".to_string());

// Restore original thread
queue.restore_thread_state(state);
```

## Preview Widget

Display pending messages in the TUI:

```rust
use limit_tui::components::{PendingInputPreview, PreviewConfig};

let config = PreviewConfig {
    edit_binding: "Ctrl+E".to_string(),
};

let mut preview = PendingInputPreview::with_config(config);
preview.set_data(
    vec!["Queued 1".to_string(), "Queued 2".to_string()],
    vec!["Pending".to_string()],
    false,
);
```

## Accessibility

The `PendingInputPreview` widget supports screen reader announcements:

```rust
// Get accessible label for screen readers
let label = preview.accessible_label();
// Returns: "2 pending messages waiting for next tool call. 3 messages queued."

// Check if changes should be announced
if preview.should_announce() {
    // Announce to screen reader
}
```

## Error Handling

```rust
use limit_tui::components::QueueError;

match queue.submit_message(msg, send_fn) {
    Ok(id) => println!("Message queued with id: {:?}", id),
    Err(QueueError::QueueFullEvicted(n)) => {
        eprintln!("Queue full, oldest of {} evicted", n);
    }
    Err(QueueError::InterruptInProgress) => {
        eprintln!("Cannot send while interrupt in progress");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```
