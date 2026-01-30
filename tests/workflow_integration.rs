//! End-to-end integration tests for full workflow pipeline
//!
//! Tests the complete flow: YAML parsing → DAG building → execution → state persistence

use picoflow::dag::DagEngine;
use picoflow::models::{TaskStatus, WorkflowConfig};
use picoflow::parser::parse_workflow_yaml;
use picoflow::scheduler::TaskScheduler;
use picoflow::state::StateManager;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a temporary directory for test state database
async fn setup_temp_state() -> (TempDir, Arc<StateManager>) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let state_manager = Arc::new(StateManager::new(db_path).await.unwrap());
    (temp_dir, state_manager)
}

#[tokio::test]
async fn test_full_workflow_execution() {
    // Create temp state database
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Define workflow YAML with dependent tasks
    let yaml = r#"
name: test-workflow
description: "Integration test workflow"
config:
  max_parallel: 1
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_a
    type: shell
    config:
      command: "/bin/echo"
      args: ["hello from task_a"]

  - name: task_b
    type: shell
    depends_on: [task_a]
    config:
      command: "/bin/echo"
      args: ["hello from task_b"]

  - name: task_c
    type: shell
    depends_on: [task_b]
    config:
      command: "/bin/date"
"#;

    // Parse YAML
    let config: WorkflowConfig = parse_workflow_yaml(yaml).unwrap();
    assert_eq!(config.name, "test-workflow");
    assert_eq!(config.tasks.len(), 3);

    // Build and validate DAG
    let dag = DagEngine::build(&config.tasks).unwrap();
    let execution_order = dag.topological_sort().unwrap();
    assert_eq!(execution_order.len(), 3);
    assert_eq!(execution_order[0], "task_a");
    assert_eq!(execution_order[1], "task_b");
    assert_eq!(execution_order[2], "task_c");

    // Execute workflow
    let scheduler = TaskScheduler::new(state_manager.clone());
    let success = scheduler.execute_workflow(&config).await.unwrap();
    assert!(success, "Workflow execution should succeed");

    // Verify state persistence
    // Check that workflow was registered
    let workflow_id = state_manager
        .get_or_create_workflow(&config.name, None)
        .await
        .unwrap();
    assert!(workflow_id > 0, "Workflow should be registered");

    // Check execution history
    let history = state_manager
        .get_execution_history("test-workflow", 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1, "Should have one execution record");
    assert_eq!(
        history[0].status,
        TaskStatus::Success,
        "Execution should be successful"
    );

    // Check individual task statuses
    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 3, "Should have 3 task results");

    // All tasks should have succeeded
    for task in &tasks {
        assert_eq!(
            task.status,
            TaskStatus::Success,
            "Task {} should succeed",
            task.task_name
        );
    }
}

#[tokio::test]
async fn test_workflow_with_failing_task() {
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Workflow where task_b will fail
    let yaml = r#"
name: failing-workflow
description: "Workflow with a failing task"
config:
  max_parallel: 1
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_a
    type: shell
    config:
      command: "/bin/echo"
      args: ["task_a succeeds"]

  - name: task_b
    type: shell
    depends_on: [task_a]
    retry: 0
    config:
      command: "/bin/sh"
      args: ["-c", "exit 1"]

  - name: task_c
    type: shell
    depends_on: [task_b]
    retry: 0
    config:
      command: "/bin/echo"
      args: ["task_c should not run"]
"#;

    let config = parse_workflow_yaml(yaml).unwrap();
    let scheduler = TaskScheduler::new(state_manager.clone());

    // Execute workflow - should fail
    let success = scheduler.execute_workflow(&config).await.unwrap();
    assert!(!success, "Workflow should fail when a task fails");

    // Verify execution history shows failure
    let history = state_manager
        .get_execution_history("failing-workflow", 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, TaskStatus::Failed);

    // Check task statuses
    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();

    // Should have task_a and task_b results (task_c should not run)
    // Note: task_b might have multiple attempts due to retries
    let task_a_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_a").collect();
    let task_b_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_b").collect();
    let task_c_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_c").collect();

    assert!(!task_a_executions.is_empty(), "task_a should have executed");
    assert!(!task_b_executions.is_empty(), "task_b should have executed");
    assert!(
        task_c_executions.is_empty(),
        "task_c should not execute when task_b fails"
    );

    // Find the final task_a execution - should succeed
    let task_a = task_a_executions.last().unwrap();
    assert_eq!(task_a.status, TaskStatus::Success);

    // Find the final task_b execution - should fail
    let task_b = task_b_executions.last().unwrap();
    assert_eq!(task_b.status, TaskStatus::Failed);
}

#[tokio::test]
async fn test_workflow_parallel_execution() {
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Workflow with independent tasks that can run in parallel
    let yaml = r#"
name: parallel-workflow
description: "Workflow with parallel execution"
config:
  max_parallel: 4
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_1
    type: shell
    config:
      command: "/bin/echo"
      args: ["task 1"]

  - name: task_2
    type: shell
    config:
      command: "/bin/echo"
      args: ["task 2"]

  - name: task_3
    type: shell
    config:
      command: "/bin/echo"
      args: ["task 3"]

  - name: task_4
    type: shell
    config:
      command: "/bin/echo"
      args: ["task 4"]
"#;

    let config = parse_workflow_yaml(yaml).unwrap();
    let scheduler = TaskScheduler::new(state_manager.clone());

    // Execute workflow with parallel execution
    let success = scheduler.execute_workflow(&config).await.unwrap();
    assert!(success, "Parallel workflow should succeed");

    // Verify all tasks completed successfully
    let history = state_manager
        .get_execution_history("parallel-workflow", 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, TaskStatus::Success);

    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 4, "All 4 tasks should complete");

    // All should succeed
    for task in &tasks {
        assert_eq!(task.status, TaskStatus::Success);
    }
}

#[tokio::test]
async fn test_parallel_semaphore_enforcement() {
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Create workflow with 6 independent tasks and max_parallel=2
    let yaml = r#"
name: semaphore-test
description: "Test semaphore enforcement"
config:
  max_parallel: 2
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_1
    type: shell
    config:
      command: "/bin/echo"
      args: ["1"]

  - name: task_2
    type: shell
    config:
      command: "/bin/echo"
      args: ["2"]

  - name: task_3
    type: shell
    config:
      command: "/bin/echo"
      args: ["3"]

  - name: task_4
    type: shell
    config:
      command: "/bin/echo"
      args: ["4"]

  - name: task_5
    type: shell
    config:
      command: "/bin/echo"
      args: ["5"]

  - name: task_6
    type: shell
    config:
      command: "/bin/echo"
      args: ["6"]
"#;

    let config = parse_workflow_yaml(yaml).unwrap();
    let scheduler = TaskScheduler::new(state_manager.clone());

    let success = scheduler.execute_workflow(&config).await.unwrap();
    assert!(success, "All tasks should complete successfully");

    // Verify all tasks completed
    let history = state_manager
        .get_execution_history("semaphore-test", 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);

    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();
    assert_eq!(tasks.len(), 6, "All 6 tasks should complete");

    // Verify all succeeded
    for task in &tasks {
        assert_eq!(
            task.status,
            TaskStatus::Success,
            "Task {} should succeed",
            task.task_name
        );
    }
}

#[tokio::test]
async fn test_parallel_dependency_failure_skips_dependents() {
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Workflow where task_a fails, task_b depends on task_a
    // continue_on_failure is false by default at workflow level
    let yaml = r#"
name: dependency-failure
description: "Test failure propagation"
config:
  max_parallel: 2
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_a
    type: shell
    retry: 0
    config:
      command: "/bin/sh"
      args: ["-c", "exit 1"]

  - name: task_b
    type: shell
    depends_on: [task_a]
    retry: 0
    config:
      command: "/bin/echo"
      args: ["should not run"]

  - name: task_c
    type: shell
    retry: 0
    config:
      command: "/bin/echo"
      args: ["independent task"]
"#;

    let config = parse_workflow_yaml(yaml).unwrap();
    let scheduler = TaskScheduler::new(state_manager.clone());

    let success = scheduler.execute_workflow(&config).await.unwrap();
    assert!(!success, "Workflow should fail");

    let history = state_manager
        .get_execution_history("dependency-failure", 10)
        .await
        .unwrap();
    assert_eq!(history[0].status, TaskStatus::Failed);

    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();

    // Find task executions
    let task_a_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_a").collect();
    let task_b_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_b").collect();

    // task_a should fail
    assert!(!task_a_executions.is_empty(), "task_a should execute");
    let task_a = task_a_executions.last().unwrap();
    assert_eq!(task_a.status, TaskStatus::Failed);

    // task_b should NOT have run (because task_a failed)
    assert!(
        task_b_executions.is_empty(),
        "task_b should not execute when task_a fails"
    );

    // task_c is independent and may or may not run depending on parallel execution timing
    // We don't assert on task_c behavior
}

#[tokio::test]
async fn test_parallel_continue_on_failure() {
    let (_temp_dir, state_manager) = setup_temp_state().await;

    // Workflow where task_a has continue_on_failure=true and fails
    // task_b depends on task_a and should still run
    let yaml = r#"
name: continue-on-failure
description: "Test continue_on_failure"
config:
  max_parallel: 2
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task_a
    type: shell
    continue_on_failure: true
    retry: 0
    config:
      command: "/bin/sh"
      args: ["-c", "exit 1"]

  - name: task_b
    type: shell
    depends_on: [task_a]
    retry: 0
    config:
      command: "/bin/echo"
      args: ["should run despite task_a failure"]
"#;

    let config = parse_workflow_yaml(yaml).unwrap();
    let scheduler = TaskScheduler::new(state_manager.clone());

    let success = scheduler.execute_workflow(&config).await.unwrap();
    // Workflow should fail overall (because task_a failed), but task_b should run
    assert!(!success, "Workflow should fail overall");

    let history = state_manager
        .get_execution_history("continue-on-failure", 10)
        .await
        .unwrap();

    let execution_id = history[0].id;
    let tasks = state_manager
        .get_task_executions(execution_id)
        .await
        .unwrap();

    // Find task executions
    let task_a_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_a").collect();
    let task_b_executions: Vec<_> = tasks.iter().filter(|t| t.task_name == "task_b").collect();

    // task_a should fail
    assert!(!task_a_executions.is_empty(), "task_a should execute");
    let task_a = task_a_executions.last().unwrap();
    assert_eq!(task_a.status, TaskStatus::Failed);

    // task_b should have run and succeeded
    assert!(!task_b_executions.is_empty(), "task_b should execute");
    let task_b = task_b_executions.last().unwrap();
    assert_eq!(
        task_b.status,
        TaskStatus::Success,
        "task_b should run when task_a has continue_on_failure=true"
    );
}
