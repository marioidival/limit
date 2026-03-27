// Input Queue System for managing user input during async operations
// Based on Codex's two-tier queue design with explicit state machine

use std::collections::VecDeque;
use std::time::{Duration, Instant};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum queued messages before eviction (backpressure)
pub const MAX_QUEUED_MESSAGES: usize = 50;

/// Maximum pending steers before blocking new sends
pub const MAX_PENDING_STEERS: usize = 10;

/// Timeout for double-ESC force cancel
pub const INTERRUPT_TIMEOUT_MS: u64 = 500;

// ============================================================================
// CORE MESSAGE TYPES
// ============================================================================

/// Unique identifier for a message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageId(pub u64);

impl Default for MessageId {
    fn default() -> Self {
        Self(
            blake3::hash(&Instant::now().elapsed().as_nanos().to_le_bytes()).as_bytes()[0..8]
                .try_into()
                .map(u64::from_le_bytes)
                .unwrap_or(0),
        )
    }
}

/// A text span with byte range for precise positioning
#[derive(Debug, Clone, PartialEq)]
pub struct TextElement {
    pub text: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub element_type: TextElementType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextElementType {
    Plain,
    Code { language: Option<String> },
    Mention { entity_id: String },
}

/// Local image attachment with path and metadata
#[derive(Debug, Clone)]
pub struct LocalImageAttachment {
    pub path: std::path::PathBuf,
    pub mime_type: String,
    pub byte_size: u64,
}

/// Binding between a mention and its resolved entity
#[derive(Debug, Clone)]
pub struct MentionBinding {
    pub mention_text: String,
    pub byte_range: (usize, usize),
    pub resolved_entity_id: String,
    pub resolved_name: String,
}

/// User input message with all attachment types
#[derive(Debug, Clone)]
pub struct UserMessage {
    pub text: String,
    pub local_images: Vec<LocalImageAttachment>,
    pub remote_image_urls: Vec<String>,
    pub text_elements: Vec<TextElement>,
    pub mention_bindings: Vec<MentionBinding>,
    pub created_at: Instant,
    pub id: MessageId,
}

impl Default for UserMessage {
    fn default() -> Self {
        Self {
            text: String::new(),
            local_images: Vec::new(),
            remote_image_urls: Vec::new(),
            text_elements: Vec::new(),
            mention_bindings: Vec::new(),
            created_at: Instant::now(),
            id: MessageId::default(),
        }
    }
}

impl UserMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            id: MessageId::default(),
            created_at: Instant::now(),
            ..Default::default()
        }
    }

    /// Add image attachment, returns mutable self for chaining
    pub fn with_local_image(mut self, path: std::path::PathBuf, mime_type: String) -> Self {
        self.local_images.push(LocalImageAttachment {
            byte_size: 0,
            path,
            mime_type,
        });
        self
    }

    /// Total byte size of all attachments
    pub fn total_attachment_size(&self) -> u64 {
        self.local_images.iter().map(|i| i.byte_size).sum()
    }

    /// Truncate text for preview display
    pub fn preview_text(&self, max_lines: usize) -> String {
        let lines: Vec<&str> = self.text.lines().take(max_lines).collect();
        let truncated = lines.join("\n");
        if self.text.lines().count() > max_lines {
            format!("{}…", truncated)
        } else {
            truncated
        }
    }

    /// Merge another message into this one
    pub fn merge(&mut self, other: UserMessage) {
        let base_len = self.text.len();

        // Merge text
        if !self.text.is_empty() && !other.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(&other.text);

        // Adjust and merge text elements
        for mut elem in other.text_elements {
            elem.byte_start += base_len;
            elem.byte_end += base_len;
            self.text_elements.push(elem);
        }

        // Merge attachments (dedupe by path)
        for img in other.local_images {
            if !self.local_images.iter().any(|i| i.path == img.path) {
                self.local_images.push(img);
            }
        }

        // Merge remote URLs (dedupe)
        for url in other.remote_image_urls {
            if !self.remote_image_urls.contains(&url) {
                self.remote_image_urls.push(url);
            }
        }

        // Merge mentions (dedupe by resolved entity)
        for mention in other.mention_bindings {
            if !self
                .mention_bindings
                .iter()
                .any(|m| m.resolved_entity_id == mention.resolved_entity_id)
            {
                let mut mention = mention;
                mention.byte_range.0 += base_len;
                mention.byte_range.1 += base_len;
                self.mention_bindings.push(mention);
            }
        }
    }
}

// ============================================================================
// PENDING STEER (Submitted but not committed)
// ============================================================================

/// Key for comparing/deduplicating pending steers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingSteerKey {
    pub message_id: MessageId,
    pub submit_order: u64,
}

#[derive(Debug, Clone)]
pub struct PendingSteer {
    pub user_message: UserMessage,
    pub compare_key: PendingSteerKey,
    pub submitted_at: Instant,
    pub interrupt_sent: bool,
}

// ============================================================================
// INPUT STATE MACHINE
// ============================================================================

/// Explicit state machine for input handling
///
/// # State Transitions
/// ```text
/// Idle ──(task starts)──> TaskRunning
/// TaskRunning ──(task completes)──> Idle
/// TaskRunning ──(ESC pressed)──> Interrupting
/// Interrupting ──(interrupt ack)──> TaskRunning or Idle
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub enum InputState {
    /// No task running, input sends immediately
    #[default]
    Idle,

    /// Task is executing, input queues
    TaskRunning {
        /// Whether interrupt is currently possible
        can_interrupt: bool,
        /// Number of pending steers already submitted
        pending_count: usize,
    },

    /// Interrupt has been sent, awaiting acknowledgment
    Interrupting {
        /// When the interrupt was sent (for timeout detection)
        interrupt_sent_at: Instant,
        /// Steers that triggered this interrupt
        triggered_by: Vec<MessageId>,
    },
}

impl InputState {
    /// Check if new messages should queue (vs send immediately)
    pub fn should_queue(&self) -> bool {
        matches!(self, Self::TaskRunning { .. } | Self::Interrupting { .. })
    }

    /// Check if interrupt can be sent now
    pub fn can_interrupt(&self) -> bool {
        match self {
            Self::TaskRunning { can_interrupt, .. } => *can_interrupt,
            Self::Interrupting { .. } => false,
            Self::Idle => false,
        }
    }

    /// Check if a new interrupt request should be ignored (spam protection)
    pub fn is_interrupt_in_progress(&self) -> bool {
        matches!(self, Self::Interrupting { .. })
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue at capacity ({0} messages), oldest evicted")]
    QueueFullEvicted(usize),

    #[error("Cannot send while interrupt in progress")]
    InterruptInProgress,

    #[error("Message with id {0:?} not found in queue")]
    MessageNotFound(MessageId),

    #[error("Invalid state transition")]
    InvalidTransition,

    #[error("Failed to send message: {0}")]
    SendFailed(String),
}

// ============================================================================
// ESC ACTION RESULT
// ============================================================================

/// Result of handling ESC key press
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EscActionResult {
    /// Interrupt was sent successfully
    InterruptSent,
    /// Interrupt is already in progress (idempotent)
    InterruptInProgress,
    /// Interrupt is not available (task doesn't support it)
    InterruptNotAvailable,
    /// Force cancelled after double-ESC timeout
    ForceCancelled,
    /// Nothing to interrupt (idle state)
    NothingToInterrupt,
}

// ============================================================================
// MAIN INPUT QUEUE MANAGER
// ============================================================================

/// Manager for the two-tier input queue system
#[derive(Debug)]
pub struct InputQueueManager {
    /// Messages not yet submitted
    queued_messages: VecDeque<UserMessage>,

    /// Messages submitted but not committed by backend
    pending_steers: VecDeque<PendingSteer>,

    /// Current input state
    state: InputState,

    /// Monotonic counter for ordering
    submit_counter: u64,

    /// Flag to suppress auto-send (e.g., during edit operations)
    suppress_autosend: bool,
}

impl Default for InputQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputQueueManager {
    pub fn new() -> Self {
        Self {
            queued_messages: VecDeque::with_capacity(MAX_QUEUED_MESSAGES),
            pending_steers: VecDeque::with_capacity(MAX_PENDING_STEERS),
            state: InputState::Idle,
            submit_counter: 0,
            suppress_autosend: false,
        }
    }

    // ========================================================================
    // CORE OPERATIONS
    // ========================================================================

    /// Submit a new user message
    /// Returns the message ID if successfully queued or sent
    pub fn submit_message<F>(
        &mut self,
        message: UserMessage,
        mut send_fn: F,
    ) -> Result<MessageId, QueueError>
    where
        F: FnMut(&UserMessage) -> Result<(), String>,
    {
        let message_id = message.id;

        match &self.state {
            InputState::Idle => {
                send_fn(&message).map_err(QueueError::SendFailed)?;
                self.add_pending_steer(message);
                Ok(message_id)
            }

            InputState::TaskRunning { .. } | InputState::Interrupting { .. } => {
                self.enqueue_with_backpressure(message)?;
                Ok(message_id)
            }
        }
    }

    /// Internal: Add message to pending steers
    fn add_pending_steer(&mut self, message: UserMessage) {
        let steer = PendingSteer {
            compare_key: PendingSteerKey {
                message_id: message.id,
                submit_order: self.submit_counter,
            },
            user_message: message,
            submitted_at: Instant::now(),
            interrupt_sent: false,
        };
        self.submit_counter += 1;
        self.pending_steers.push_back(steer);
    }

    /// Internal: Enqueue with backpressure handling
    fn enqueue_with_backpressure(&mut self, message: UserMessage) -> Result<(), QueueError> {
        if self.queued_messages.len() >= MAX_QUEUED_MESSAGES {
            let evicted = self.queued_messages.pop_front();
            tracing::warn!(
                evicted_id = ?evicted.map(|m| m.id),
                "Queue full, evicted oldest message"
            );
        }
        self.queued_messages.push_back(message);
        Ok(())
    }

    pub fn request_interrupt(&mut self) -> Result<EscActionResult, QueueError> {
        match self.state.clone() {
            InputState::TaskRunning {
                can_interrupt,
                pending_count,
            } => {
                if !can_interrupt {
                    return Ok(EscActionResult::InterruptNotAvailable);
                }

                let triggered_by: Vec<MessageId> = self
                    .pending_steers
                    .iter()
                    .map(|s| s.user_message.id)
                    .collect();

                self.state = InputState::Interrupting {
                    interrupt_sent_at: Instant::now(),
                    triggered_by,
                };

                for steer in &mut self.pending_steers {
                    steer.interrupt_sent = true;
                }

                tracing::info!(pending_count, "Interrupt requested");
                Ok(EscActionResult::InterruptSent)
            }

            InputState::Interrupting {
                interrupt_sent_at, ..
            } => {
                let elapsed = interrupt_sent_at.elapsed();
                if elapsed > Duration::from_millis(INTERRUPT_TIMEOUT_MS) {
                    self.state = InputState::Idle;
                    self.pending_steers.clear();
                    Ok(EscActionResult::ForceCancelled)
                } else {
                    Ok(EscActionResult::InterruptInProgress)
                }
            }

            InputState::Idle => Ok(EscActionResult::NothingToInterrupt),
        }
    }

    /// Called when task starts running
    pub fn on_task_started(&mut self, can_interrupt: bool) {
        self.state = InputState::TaskRunning {
            can_interrupt,
            pending_count: self.pending_steers.len(),
        };
    }

    /// Called when task completes (successfully or aborted)
    /// Returns the next message to send if any
    pub fn on_task_completed<F>(
        &mut self,
        mut send_fn: F,
    ) -> Result<Option<UserMessage>, QueueError>
    where
        F: FnMut(&UserMessage) -> Result<(), String>,
    {
        self.state = InputState::Idle;
        self.maybe_send_next_queued(&mut send_fn)
    }

    /// Called when interrupt is acknowledged by backend
    pub fn on_interrupt_acknowledged<F>(
        &mut self,
        mut send_fn: F,
    ) -> Result<Option<UserMessage>, QueueError>
    where
        F: FnMut(&UserMessage) -> Result<(), String>,
    {
        // After interrupt ack, send pending steers immediately
        let steers_to_send: Vec<UserMessage> = self
            .pending_steers
            .drain(..)
            .map(|s| s.user_message)
            .collect();

        if !steers_to_send.is_empty() {
            // Merge all steers into one message
            let mut merged = UserMessage::new(String::new());
            for steer in steers_to_send {
                merged.merge(steer);
            }

            // Send the merged message
            send_fn(&merged).map_err(QueueError::SendFailed)?;

            // Transition back to running or idle
            self.state = InputState::TaskRunning {
                can_interrupt: true,
                pending_count: 0,
            };

            return Ok(Some(merged));
        }

        // No steers to send, just transition state
        self.state = InputState::TaskRunning {
            can_interrupt: true,
            pending_count: 0,
        };

        self.maybe_send_next_queued(&mut send_fn)
    }

    /// Pop last queued message for editing (Alt+Up)
    pub fn pop_for_edit(&mut self) -> Option<UserMessage> {
        self.queued_messages.pop_back()
    }

    /// Send next queued message if appropriate
    pub fn maybe_send_next_queued<F>(
        &mut self,
        mut send_fn: F,
    ) -> Result<Option<UserMessage>, QueueError>
    where
        F: FnMut(&UserMessage) -> Result<(), String>,
    {
        if self.suppress_autosend || self.state.should_queue() {
            return Ok(None);
        }

        if let Some(message) = self.queued_messages.pop_front() {
            send_fn(&message).map_err(QueueError::SendFailed)?;
            self.add_pending_steer(message.clone());
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    pub fn set_suppress_autosend(&mut self, suppress: bool) {
        self.suppress_autosend = suppress;
    }

    pub fn is_autosend_suppressed(&self) -> bool {
        self.suppress_autosend
    }

    /// Commit pending steers (called when backend confirms receipt)
    pub fn commit_pending_steers(&mut self) -> Vec<UserMessage> {
        self.pending_steers
            .drain(..)
            .map(|s| s.user_message)
            .collect()
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    pub fn queued_count(&self) -> usize {
        self.queued_messages.len()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_steers.len()
    }

    pub fn current_state(&self) -> &InputState {
        &self.state
    }

    pub fn queued_messages(&self) -> impl Iterator<Item = &UserMessage> {
        self.queued_messages.iter()
    }

    pub fn pending_steers(&self) -> impl Iterator<Item = &PendingSteer> {
        self.pending_steers.iter()
    }

    /// Check if Alt+Up (edit queued) is available
    pub fn can_edit_queued(&self) -> bool {
        !self.queued_messages.is_empty()
    }

    /// Check if there are any pending messages
    pub fn has_pending(&self) -> bool {
        !self.queued_messages.is_empty() || !self.pending_steers.is_empty()
    }

    /// Get preview data for the pending input widget
    pub fn preview_data(&self) -> (Vec<String>, Vec<String>) {
        let queued: Vec<String> = self
            .queued_messages
            .iter()
            .map(|m| m.text.clone())
            .collect();
        let pending: Vec<String> = self
            .pending_steers
            .iter()
            .map(|s| s.user_message.text.clone())
            .collect();
        (queued, pending)
    }

    /// Clear all queues
    pub fn clear(&mut self) {
        self.queued_messages.clear();
        self.pending_steers.clear();
        self.state = InputState::Idle;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message_new() {
        let msg = UserMessage::new("Hello".to_string());
        assert_eq!(msg.text, "Hello");
        assert!(msg.local_images.is_empty());
        assert!(msg.remote_image_urls.is_empty());
    }

    #[test]
    fn test_user_message_preview_text() {
        let msg = UserMessage::new("Line1\nLine2\nLine3\nLine4".to_string());
        let preview = msg.preview_text(2);
        assert!(preview.contains("Line1"));
        assert!(preview.contains("Line2"));
        assert!(preview.contains("…"));
    }

    #[test]
    fn test_user_message_merge() {
        let mut msg1 = UserMessage::new("Hello".to_string());
        let msg2 = UserMessage::new("World".to_string());

        msg1.merge(msg2);

        assert_eq!(msg1.text, "Hello\nWorld");
    }

    #[test]
    fn test_input_state_should_queue() {
        assert!(!InputState::Idle.should_queue());
        assert!(InputState::TaskRunning {
            can_interrupt: true,
            pending_count: 0
        }
        .should_queue());
        assert!(InputState::Interrupting {
            interrupt_sent_at: Instant::now(),
            triggered_by: vec![]
        }
        .should_queue());
    }

    #[test]
    fn test_input_state_can_interrupt() {
        assert!(!InputState::Idle.can_interrupt());
        assert!(InputState::TaskRunning {
            can_interrupt: true,
            pending_count: 0
        }
        .can_interrupt());
        assert!(!InputState::TaskRunning {
            can_interrupt: false,
            pending_count: 0
        }
        .can_interrupt());
    }

    #[test]
    fn test_queue_manager_new() {
        let manager = InputQueueManager::new();
        assert_eq!(manager.queued_count(), 0);
        assert_eq!(manager.pending_count(), 0);
        assert!(!manager.has_pending());
        assert_eq!(manager.current_state(), &InputState::Idle);
    }

    #[test]
    fn test_queue_manager_submit_idle() {
        let mut manager = InputQueueManager::new();
        let msg = UserMessage::new("Test".to_string());

        let result = manager.submit_message(msg, |_| Ok(()));
        assert!(result.is_ok());

        // Message should be in pending steers
        assert_eq!(manager.pending_count(), 1);
        assert_eq!(manager.queued_count(), 0);
    }

    #[test]
    fn test_queue_manager_submit_running() {
        let mut manager = InputQueueManager::new();
        manager.on_task_started(true);

        let msg = UserMessage::new("Test".to_string());
        let result = manager.submit_message(msg, |_| Ok(()));
        assert!(result.is_ok());

        // Message should be queued, not sent
        assert_eq!(manager.queued_count(), 1);
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_queue_manager_interrupt() {
        let mut manager = InputQueueManager::new();
        manager.on_task_started(true);

        // First interrupt
        let result = manager.request_interrupt();
        assert_eq!(result.unwrap(), EscActionResult::InterruptSent);
        assert!(manager.current_state().is_interrupt_in_progress());

        // Second interrupt (idempotent)
        let result = manager.request_interrupt();
        assert_eq!(result.unwrap(), EscActionResult::InterruptInProgress);
    }

    #[test]
    fn test_queue_manager_pop_for_edit() {
        let mut manager = InputQueueManager::new();
        manager.on_task_started(true);

        let msg1 = UserMessage::new("First".to_string());
        let msg2 = UserMessage::new("Second".to_string());

        manager.submit_message(msg1, |_| Ok(())).unwrap();
        manager.submit_message(msg2, |_| Ok(())).unwrap();

        // Pop last
        let popped = manager.pop_for_edit();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().text, "Second");
        assert_eq!(manager.queued_count(), 1);
    }

    #[test]
    fn test_queue_manager_task_completed() {
        let mut manager = InputQueueManager::new();
        manager.on_task_started(true);

        let msg = UserMessage::new("Test".to_string());
        manager.submit_message(msg, |_| Ok(())).unwrap();

        assert_eq!(manager.queued_count(), 1);

        // Task completes
        let result = manager.on_task_completed(|_| Ok(()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
        assert_eq!(manager.queued_count(), 0);
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_backpressure_eviction() {
        let mut manager = InputQueueManager::new();
        manager.on_task_started(true);

        // Fill beyond capacity
        for i in 0..(MAX_QUEUED_MESSAGES + 10) {
            let msg = UserMessage::new(format!("Message {}", i));
            manager.submit_message(msg, |_| Ok(())).unwrap();
        }

        // Should be at max capacity
        assert_eq!(manager.queued_count(), MAX_QUEUED_MESSAGES);
    }

    #[test]
    fn test_preview_data() {
        let mut manager = InputQueueManager::new();

        // Add pending steer
        let msg = UserMessage::new("Pending".to_string());
        manager.submit_message(msg, |_| Ok(())).unwrap();

        // Add queued message
        manager.on_task_started(true);
        let msg2 = UserMessage::new("Queued".to_string());
        manager.submit_message(msg2, |_| Ok(())).unwrap();

        let (queued, pending) = manager.preview_data();
        assert_eq!(queued.len(), 1);
        assert_eq!(pending.len(), 1);
        assert_eq!(queued[0], "Queued");
        assert_eq!(pending[0], "Pending");
    }
}
