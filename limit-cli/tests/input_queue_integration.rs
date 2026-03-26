use limit_cli::tui::input_queue::InputQueue;

#[test]
fn test_full_queue_flow() {
    let mut queue = InputQueue::new();

    queue.queue_message("First message".to_string());
    queue.queue_message("Second message".to_string());

    assert!(queue.has_queued_messages());
    assert_eq!(queue.queued_count(), 2);

    let first = queue.pop_queued();
    assert_eq!(first.unwrap().text, "First message");
    assert_eq!(queue.queued_count(), 1);

    queue.clear();
    assert!(queue.is_empty());
}

#[test]
fn test_edit_queued_message() {
    let mut queue = InputQueue::new();

    queue.queue_message("First".to_string());
    queue.queue_message("Second".to_string());
    queue.queue_message("Third".to_string());

    let last = queue.pop_last_queued();
    assert_eq!(last.unwrap().text, "Third");
    assert_eq!(queue.queued_count(), 2);
}

#[test]
fn test_merge_all_messages() {
    let mut queue = InputQueue::new();

    queue.add_steer("Steer message".to_string());
    queue.queue_message("Queued message".to_string());

    let merged = queue.merge_all();
    assert_eq!(merged, Some("Steer message\n\nQueued message".to_string()));
    assert!(queue.is_empty());
}

#[test]
fn test_suppress_and_resume() {
    let mut queue = InputQueue::new();
    assert!(!queue.is_autosend_suppressed());

    queue.set_suppress_autosend(true);
    assert!(queue.is_autosend_suppressed());

    queue.set_suppress_autosend(false);
    assert!(!queue.is_autosend_suppressed());
}

#[test]
fn test_thread_switching() {
    let mut queue = InputQueue::new();

    queue.queue_message("Thread A message".to_string());
    queue.add_steer("Thread A steer".to_string());

    let thread_a_state = queue.save_thread_state();
    assert!(thread_a_state.is_some());

    queue.clear();

    queue.queue_message("Thread B message".to_string());
    assert_eq!(queue.queued_count(), 1);

    queue.restore_thread_state(thread_a_state);
    assert_eq!(queue.queued_count(), 1);
    assert_eq!(queue.steer_count(), 1);
}

#[test]
fn test_interrupt_flag_flow() {
    let mut queue = InputQueue::new();
    assert!(!queue.should_submit_after_interrupt());

    queue.add_steer("Steer".to_string());
    queue.set_submit_after_interrupt(true);

    assert!(queue.should_submit_after_interrupt());

    let _ = queue.merge_all();
    assert!(!queue.should_submit_after_interrupt());
}

#[test]
fn test_fifo_order() {
    let mut queue = InputQueue::new();

    for i in 1..=5 {
        queue.queue_message(format!("Message {}", i));
    }

    for i in 1..=5 {
        let msg = queue.pop_queued();
        assert_eq!(msg.unwrap().text, format!("Message {}", i));
    }

    assert!(queue.is_empty());
}

#[test]
fn test_lifo_edit_order() {
    let mut queue = InputQueue::new();

    queue.queue_message("First".to_string());
    queue.queue_message("Second".to_string());
    queue.queue_message("Third".to_string());

    assert_eq!(queue.pop_last_queued().unwrap().text, "Third");
    assert_eq!(queue.pop_last_queued().unwrap().text, "Second");
    assert_eq!(queue.pop_last_queued().unwrap().text, "First");
    assert!(queue.is_empty());
}

#[test]
fn test_drain_operations() {
    let mut queue = InputQueue::new();

    queue.queue_message("Q1".to_string());
    queue.queue_message("Q2".to_string());
    queue.add_steer("S1".to_string());

    let queued = queue.drain_queued();
    assert_eq!(queued.len(), 2);
    assert!(!queue.has_queued_messages());
    assert!(queue.has_pending_steers());

    let steers = queue.drain_steers();
    assert_eq!(steers.len(), 1);
    assert!(queue.is_empty());
}

#[test]
fn test_preview_texts() {
    let mut queue = InputQueue::new();

    queue.queue_message("Queued 1".to_string());
    queue.queue_message("Queued 2".to_string());
    queue.add_steer("Steer 1".to_string());

    let queued_texts = queue.queued_texts();
    assert_eq!(queued_texts, vec!["Queued 1", "Queued 2"]);

    let steer_texts = queue.steer_texts();
    assert_eq!(steer_texts, vec!["Steer 1"]);
}
