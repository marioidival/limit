//! Integration tests for the full team workflow with MockLlmProvider.
//!
//! Tests the complete PM → TL → Jr pipeline using a mock provider
//! that returns pre-configured responses, validating that:
//! - All 6 phases execute in order
//! - Task parsing and parallel execution work
//! - Files are extracted from results
//! - History events are recorded per phase

use limit_agent::team::{Role, Team, TeamConfig};
use limit_agent::ToolRegistry;
use limit_llm::MockLlmProvider;
use std::sync::Arc;

/// Helper: build a MockLlmProvider with responses for a full workflow run.
///
/// The workflow calls `send()` in this order:
/// 1. PM analysis
/// 2. TL plan
/// 3. TL task breakdown
/// 4. Jr task execution (1 task → 1 call)
/// 5. TL validation
/// 6. PM delivery
fn mock_provider_for_workflow() -> MockLlmProvider {
    MockLlmProvider::new()
        // 1. PM analysis
        .with_response("This is a request to add error handling to the main module.")
        // 2. TL plan
        .with_response("Plan: Add Result<T, E> wrapper around fallible operations.")
        // 3. TL task breakdown — two tasks
        .with_response("TASK: Add error types to src/error.rs\nTASK: Wrap main function in Result")
        // 4. Jr task 1 execution
        .with_response(r#"Created file: {"path": "src/error.rs"}"#)
        // 5. Jr task 2 execution
        .with_response(r#"Modified file: {"path": "src/main.rs"}"#)
        // 6. TL validation
        .with_response("Validation passed. Error handling looks correct.")
        // 7. PM delivery
        .with_response("✅ Added error handling to the project.")
}

/// Build a tool registry with an echo tool for testing.
fn test_registry() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new())
}

/// Build a Team with mock provider and default config.
fn mock_team(name: &str) -> Team {
    let config = TeamConfig::default();
    let provider: Box<dyn limit_llm::LlmProvider> = Box::new(mock_provider_for_workflow());
    let tools = test_registry();
    Team::new(name.to_string(), provider, config, tools).expect("team creation")
}

#[tokio::test]
async fn test_full_team_workflow() {
    let mut team = mock_team("full-workflow-test");
    let result = team.execute("Add error handling", None).await;

    assert!(result.is_ok(), "workflow failed: {:?}", result.err());
    let result = result.unwrap();

    // Check delivery
    assert!(
        !result.solution.is_empty(),
        "PM should deliver a non-empty summary"
    );

    // Check tasks
    assert_eq!(result.total_tasks, 2, "should have parsed 2 TASK lines");
    assert_eq!(result.failed_tasks, 0, "no tasks should fail with mock");

    // Check files_modified extraction
    assert_eq!(
        result.files_modified,
        vec!["src/error.rs", "src/main.rs"],
        "files should be extracted from Jr outputs"
    );

    // Check events — should have phase transitions + actual events
    let phase_events: Vec<_> = result
        .events
        .iter()
        .filter(|e| e.role == "system" && e.action.starts_with("phase:"))
        .collect();
    assert!(
        phase_events.len() >= 5,
        "expected at least 5 phase events, got {}: {:?}",
        phase_events.len(),
        phase_events
    );
}

#[tokio::test]
async fn test_workflow_phases_in_order() {
    let mut team = mock_team("phases-test");
    let result = team.execute("test", None).await.unwrap();

    let phase_events: Vec<_> = result
        .events
        .iter()
        .filter(|e| e.action.starts_with("phase:"))
        .collect();

    let expected_phases = [
        "phase:PmAnalysis",
        "phase:TlPlan",
        "phase:TlBreakdown",
        "phase:JrExecution",
        "phase:TlValidation",
        "phase:PmDelivery",
    ];

    for (i, expected) in expected_phases.iter().enumerate() {
        assert_eq!(
            phase_events[i].action, *expected,
            "phase at index {i} should be {expected}"
        );
    }
}

#[tokio::test]
async fn test_workflow_records_pm_analysis_event() {
    let mut team = mock_team("pm-event-test");
    let result = team.execute("test", None).await.unwrap();

    let pm_events: Vec<_> = result.events.iter().filter(|e| e.role == "PM").collect();
    assert!(
        !pm_events.is_empty(),
        "PM should have at least one recorded event"
    );
    // PM events should include "analysis" and "delivery" actions
    let actions: Vec<&str> = pm_events.iter().map(|e| e.action.as_str()).collect();
    assert!(
        actions.contains(&"analysis"),
        "PM should have analysis event"
    );
    assert!(
        actions.contains(&"delivery"),
        "PM should have delivery event"
    );
}

#[tokio::test]
async fn test_workflow_records_tl_events() {
    let mut team = mock_team("tl-event-test");
    let result = team.execute("test", None).await.unwrap();

    let tl_events: Vec<_> = result.events.iter().filter(|e| e.role == "TL").collect();
    let actions: Vec<&str> = tl_events.iter().map(|e| e.action.as_str()).collect();
    assert!(actions.contains(&"plan"), "TL should have plan event");
    assert!(actions.contains(&"tasks"), "TL should have tasks event");
    assert!(
        actions.contains(&"validation"),
        "TL should have validation event"
    );
}

#[tokio::test]
async fn test_workflow_records_jr_events() {
    let mut team = mock_team("jr-event-test");
    let result = team.execute("test", None).await.unwrap();

    let jr_events: Vec<_> = result.events.iter().filter(|e| e.role == "Jr").collect();
    assert!(
        !jr_events.is_empty(),
        "Jr should have execution results recorded"
    );
    assert_eq!(jr_events[0].action, "execution");
}

#[tokio::test]
async fn test_workflow_duration_is_positive() {
    let mut team = mock_team("duration-test");
    let result = team.execute("test", None).await.unwrap();

    assert!(
        result.duration.as_millis() > 0,
        "workflow should take non-zero time"
    );
}

#[tokio::test]
async fn test_workflow_empty_tasks_returns_early() {
    // Provider returns no parseable tasks → early delivery
    let provider: Box<dyn limit_llm::LlmProvider> = Box::new(
        MockLlmProvider::new()
            .with_response("Simple request")
            .with_response("Plan: do something")
            // No TASK: lines → empty task list
            .with_response("No specific tasks needed, just do it.")
            .with_response("Done."),
    );
    let config = TeamConfig::default();
    let tools = test_registry();
    let mut team =
        Team::new("empty-tasks-test".into(), provider, config, tools).expect("team creation");

    let result = team.execute("simple request", None).await.unwrap();
    assert_eq!(result.total_tasks, 0, "no tasks should be parsed");
    assert_eq!(result.failed_tasks, 0);
    assert!(
        !result.solution.is_empty(),
        "should still deliver a summary"
    );
}

#[tokio::test]
async fn test_team_create_and_events() {
    let team = mock_team("events-test");

    // Check team structure
    assert_eq!(team.name, "events-test");
    assert_eq!(team.jrs.len(), 2, "default config has 2 juniors");
    assert_eq!(team.pm.role(), &Role::PM);
    assert_eq!(team.tl.role(), &Role::TL);
    assert!(team.jrs.iter().all(|jr| jr.role() == &Role::Jr));
}

#[tokio::test]
async fn test_team_reset_clears_state() {
    let mut team = mock_team("reset-test");
    team.execute("test", None).await.unwrap();

    // After execution, there should be events
    let events_before = team.events().await;
    assert!(!events_before.is_empty());

    // Reset
    team.reset().await;

    let events_after = team.events().await;
    assert!(
        events_after.is_empty(),
        "events should be cleared after reset"
    );
}

#[tokio::test]
async fn test_team_persistence_save_load() {
    let team = mock_team("persist-test");

    // Export history + config to a snapshot
    let snapshot = limit_agent::team::TeamSnapshot::new("persist-test", team.config.clone());

    // Serialize and deserialize
    let json = serde_json::to_string(&snapshot).unwrap();
    let loaded: limit_agent::team::TeamSnapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.name, "persist-test");
    assert_eq!(loaded.config.num_juniors, team.config.num_juniors);
}
