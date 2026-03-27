// Input queue system for managing messages during async operations

use std::collections::VecDeque;

const DEFAULT_MAX_QUEUED_MESSAGES: usize = 50;
const DEFAULT_MAX_PENDING_STEERS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueConfig {
    pub max_queued_messages: usize,
    pub max_pending_steers: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queued_messages: DEFAULT_MAX_QUEUED_MESSAGES,
            max_pending_steers: DEFAULT_MAX_PENDING_STEERS,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedMessage {
    pub text: String,
    pub is_steer: bool,
}

impl QueuedMessage {
    pub fn new(text: String) -> Self {
        Self {
            text,
            is_steer: false,
        }
    }

    pub fn new_steer(text: String) -> Self {
        Self {
            text,
            is_steer: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThreadInputState {
    pub queued_messages: Vec<QueuedMessage>,
    pub pending_steers: Vec<QueuedMessage>,
    pub submit_after_interrupt: bool,
    pub suppress_autosend: bool,
}

impl ThreadInputState {
    pub fn has_content(&self) -> bool {
        !self.queued_messages.is_empty() || !self.pending_steers.is_empty()
    }
}

/// Manages input messages during async operations
#[derive(Debug, Clone)]
pub struct InputQueue {
    queued_messages: VecDeque<QueuedMessage>,
    pending_steers: VecDeque<QueuedMessage>,
    submit_pending_steers_after_interrupt: bool,
    suppress_autosend: bool,
    config: QueueConfig,
}

impl Default for InputQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl InputQueue {
    pub fn new() -> Self {
        Self::with_config(QueueConfig::default())
    }

    pub fn with_config(config: QueueConfig) -> Self {
        Self {
            queued_messages: VecDeque::with_capacity(config.max_queued_messages),
            pending_steers: VecDeque::with_capacity(config.max_pending_steers),
            submit_pending_steers_after_interrupt: false,
            suppress_autosend: false,
            config,
        }
    }

    pub fn queue_message(&mut self, text: String) {
        if self.queued_messages.len() >= self.config.max_queued_messages {
            self.queued_messages.pop_front();
        }
        self.queued_messages.push_back(QueuedMessage::new(text));
    }

    pub fn add_steer(&mut self, text: String) {
        if self.pending_steers.len() >= self.config.max_pending_steers {
            self.pending_steers.pop_front();
        }
        self.pending_steers
            .push_back(QueuedMessage::new_steer(text));
    }

    /// Check if there are any queued messages
    pub fn has_queued_messages(&self) -> bool {
        !self.queued_messages.is_empty()
    }

    /// Check if there are any pending steers
    pub fn has_pending_steers(&self) -> bool {
        !self.pending_steers.is_empty()
    }

    /// Check if queue is empty (no queued or pending messages)
    pub fn is_empty(&self) -> bool {
        self.queued_messages.is_empty() && self.pending_steers.is_empty()
    }

    /// Get number of queued messages
    pub fn queued_count(&self) -> usize {
        self.queued_messages.len()
    }

    /// Get number of pending steers
    pub fn steer_count(&self) -> usize {
        self.pending_steers.len()
    }

    /// Pop the next queued message (FIFO)
    pub fn pop_queued(&mut self) -> Option<QueuedMessage> {
        self.queued_messages.pop_front()
    }

    /// Get all queued message texts for preview
    pub fn queued_texts(&self) -> Vec<String> {
        self.queued_messages
            .iter()
            .map(|m| m.text.clone())
            .collect()
    }

    /// Get all pending steer texts for preview
    pub fn steer_texts(&self) -> Vec<String> {
        self.pending_steers.iter().map(|m| m.text.clone()).collect()
    }

    /// Drain all pending steers
    pub fn drain_steers(&mut self) -> Vec<QueuedMessage> {
        self.pending_steers.drain(..).collect()
    }

    /// Drain all queued messages
    pub fn drain_queued(&mut self) -> Vec<QueuedMessage> {
        self.queued_messages.drain(..).collect()
    }

    /// Pop the last queued message for editing
    pub fn pop_last_queued(&mut self) -> Option<QueuedMessage> {
        self.queued_messages.pop_back()
    }

    /// Mark that next interrupt should submit steers
    pub fn set_submit_after_interrupt(&mut self, value: bool) {
        self.submit_pending_steers_after_interrupt = value;
    }

    /// Check if should submit steers after interrupt
    pub fn should_submit_after_interrupt(&self) -> bool {
        self.submit_pending_steers_after_interrupt
    }

    /// Set suppress autosend flag
    pub fn set_suppress_autosend(&mut self, value: bool) {
        self.suppress_autosend = value;
    }

    /// Check if autosend is suppressed
    pub fn is_autosend_suppressed(&self) -> bool {
        self.suppress_autosend
    }

    /// Merge all pending messages (steers + queued) into a single text
    pub fn merge_all(&mut self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut texts: Vec<String> = Vec::new();

        for steer in self.pending_steers.drain(..) {
            texts.push(steer.text);
        }

        for msg in self.queued_messages.drain(..) {
            texts.push(msg.text);
        }

        self.submit_pending_steers_after_interrupt = false;

        Some(texts.join("\n\n"))
    }

    /// Clear all queued and pending messages
    pub fn clear(&mut self) {
        self.queued_messages.clear();
        self.pending_steers.clear();
        self.submit_pending_steers_after_interrupt = false;
    }

    pub fn save_thread_state(&self) -> Option<ThreadInputState> {
        if self.queued_messages.is_empty() && self.pending_steers.is_empty() {
            return None;
        }

        Some(ThreadInputState {
            queued_messages: self.queued_messages.iter().cloned().collect(),
            pending_steers: self.pending_steers.iter().cloned().collect(),
            submit_after_interrupt: self.submit_pending_steers_after_interrupt,
            suppress_autosend: self.suppress_autosend,
        })
    }

    pub fn restore_thread_state(&mut self, state: Option<ThreadInputState>) {
        if let Some(state) = state {
            self.queued_messages.clear();
            for msg in state.queued_messages {
                self.queued_messages.push_back(msg);
            }
            for msg in state.pending_steers {
                self.pending_steers.push_back(msg);
            }
            self.submit_pending_steers_after_interrupt = state.submit_after_interrupt;
            self.suppress_autosend = state.suppress_autosend;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_message() {
        let mut queue = InputQueue::new();
        assert!(!queue.has_queued_messages());

        queue.queue_message("Hello".to_string());
        assert!(queue.has_queued_messages());
        assert_eq!(queue.queued_count(), 1);
    }

    #[test]
    fn test_add_steer() {
        let mut queue = InputQueue::new();
        assert!(!queue.has_pending_steers());

        queue.add_steer("Continue".to_string());
        assert!(queue.has_pending_steers());
        assert_eq!(queue.steer_count(), 1);
    }

    #[test]
    fn test_pop_queued() {
        let mut queue = InputQueue::new();
        queue.queue_message("First".to_string());
        queue.queue_message("Second".to_string());

        let msg = queue.pop_queued();
        assert_eq!(msg.unwrap().text, "First");
        assert_eq!(queue.queued_count(), 1);
    }

    #[test]
    fn test_pop_last_queued() {
        let mut queue = InputQueue::new();
        queue.queue_message("First".to_string());
        queue.queue_message("Second".to_string());

        let msg = queue.pop_last_queued();
        assert_eq!(msg.unwrap().text, "Second");
        assert_eq!(queue.queued_count(), 1);
    }

    #[test]
    fn test_merge_all() {
        let mut queue = InputQueue::new();
        queue.add_steer("Steer1".to_string());
        queue.queue_message("Queue1".to_string());

        let merged = queue.merge_all();
        assert_eq!(merged, Some("Steer1\n\nQueue1".to_string()));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_interrupt_flag() {
        let mut queue = InputQueue::new();
        assert!(!queue.should_submit_after_interrupt());

        queue.set_submit_after_interrupt(true);
        assert!(queue.should_submit_after_interrupt());
    }

    #[test]
    fn test_suppress_autosend() {
        let mut queue = InputQueue::new();
        assert!(!queue.is_autosend_suppressed());

        queue.set_suppress_autosend(true);
        assert!(queue.is_autosend_suppressed());
    }

    #[test]
    fn test_clear() {
        let mut queue = InputQueue::new();
        queue.queue_message("Test".to_string());
        queue.add_steer("Steer".to_string());
        queue.set_submit_after_interrupt(true);

        queue.clear();
        assert!(queue.is_empty());
        assert!(!queue.should_submit_after_interrupt());
    }

    #[test]
    fn test_thread_state_roundtrip() {
        let mut queue = InputQueue::new();
        queue.queue_message("Queued".to_string());
        queue.add_steer("Steer".to_string());
        queue.set_submit_after_interrupt(true);
        queue.set_suppress_autosend(true);

        let state = queue.save_thread_state();
        assert!(state.is_some());
        let state = state.unwrap();
        assert!(state.has_content());
        assert_eq!(state.queued_messages.len(), 1);
        assert_eq!(state.pending_steers.len(), 1);

        queue.clear();
        assert!(queue.is_empty());

        queue.restore_thread_state(Some(state));
        assert_eq!(queue.queued_count(), 1);
        assert_eq!(queue.steer_count(), 1);
        assert!(queue.should_submit_after_interrupt());
        assert!(queue.is_autosend_suppressed());
    }

    #[test]
    fn test_thread_state_empty() {
        let queue = InputQueue::new();
        assert!(queue.save_thread_state().is_none());
    }

    #[test]
    fn test_backpressure_queued() {
        let config = QueueConfig {
            max_queued_messages: 3,
            max_pending_steers: 10,
        };
        let mut queue = InputQueue::with_config(config);

        for i in 0..5 {
            queue.queue_message(format!("Message {}", i));
        }

        assert_eq!(queue.queued_count(), 3);
        let first = queue.pop_queued().unwrap();
        assert_eq!(first.text, "Message 2");
    }

    #[test]
    fn test_backpressure_steers() {
        let config = QueueConfig {
            max_queued_messages: 50,
            max_pending_steers: 2,
        };
        let mut queue = InputQueue::with_config(config);

        for i in 0..4 {
            queue.add_steer(format!("Steer {}", i));
        }

        assert_eq!(queue.steer_count(), 2);
    }
}
