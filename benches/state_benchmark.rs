use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use picoflow::models::TaskStatus;
use picoflow::state::StateManager;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Helper to create a temporary StateManager for benchmarking
fn create_temp_state_manager() -> (StateManager, TempDir, Runtime) {
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("benchmark.db");
    let manager = rt.block_on(StateManager::new(&db_path)).unwrap();
    (manager, temp_dir, rt)
}

/// Benchmark workflow creation
fn bench_workflow_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_creation");

    group.bench_function("create_workflow", |b| {
        b.iter_batched(
            create_temp_state_manager,
            |(manager, _temp_dir, rt)| {
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow(
                        black_box("test_workflow"),
                        black_box(Some("0 2 * * *")),
                    ))
                    .unwrap();
                assert!(workflow_id > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("get_existing_workflow", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                rt.block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                (manager, temp_dir, rt)
            },
            |(manager, _temp_dir, rt)| {
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow(
                        black_box("test_workflow"),
                        black_box(Some("0 2 * * *")),
                    ))
                    .unwrap();
                assert!(workflow_id > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark execution record creation and status updates
fn bench_execution_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("execution_management");

    // Benchmark starting an execution
    group.bench_function("start_execution", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                (manager, workflow_id, temp_dir, rt)
            },
            |(manager, workflow_id, _temp_dir, rt)| {
                let execution_id = rt
                    .block_on(manager.start_execution(black_box(workflow_id)))
                    .unwrap();
                assert!(execution_id > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark updating execution status
    group.bench_function("update_execution_status", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                let execution_id = rt.block_on(manager.start_execution(workflow_id)).unwrap();
                (manager, execution_id, temp_dir, rt)
            },
            |(manager, execution_id, _temp_dir, rt)| {
                rt.block_on(manager.update_execution_status(
                    black_box(execution_id),
                    black_box(TaskStatus::Success),
                ))
                .unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark task execution record operations
fn bench_task_execution_records(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_execution_records");

    // Benchmark recording task start
    group.bench_function("start_task", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                let execution_id = rt.block_on(manager.start_execution(workflow_id)).unwrap();
                (manager, execution_id, temp_dir, rt)
            },
            |(manager, execution_id, _temp_dir, rt)| {
                let task_id = rt
                    .block_on(manager.start_task(
                        black_box(execution_id),
                        black_box("test_task"),
                        1,
                    ))
                    .unwrap();
                assert!(task_id > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark updating task status
    group.bench_function("update_task_status", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                let execution_id = rt.block_on(manager.start_execution(workflow_id)).unwrap();
                let task_id = rt
                    .block_on(manager.start_task(execution_id, "test_task", 1))
                    .unwrap();
                (manager, task_id, temp_dir, rt)
            },
            |(manager, task_id, _temp_dir, rt)| {
                rt.block_on(manager.update_task_status(
                    black_box(task_id),
                    black_box(TaskStatus::Success),
                    black_box(Some(0)),
                    black_box(Some("output")),
                    black_box(Some("")),
                ))
                .unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Note: set_task_retry requires a DateTime parameter, skipping for simplicity

    group.finish();
}

/// Benchmark execution history queries
fn bench_execution_history_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("execution_history_queries");

    // Create a database with multiple executions
    fn setup_db_with_executions(count: usize) -> (StateManager, TempDir, Runtime) {
        let (manager, temp_dir, rt) = create_temp_state_manager();
        let workflow_id = rt
            .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
            .unwrap();

        for i in 0..count {
            let execution_id = rt.block_on(manager.start_execution(workflow_id)).unwrap();
            let status = if i % 3 == 0 {
                TaskStatus::Failed
            } else {
                TaskStatus::Success
            };
            rt.block_on(manager.update_execution_status(execution_id, status))
                .unwrap();

            // Add some tasks
            for j in 0..5 {
                let task_id = rt
                    .block_on(manager.start_task(execution_id, &format!("task{}", j), 1))
                    .unwrap();
                rt.block_on(manager.update_task_status(
                    task_id,
                    TaskStatus::Success,
                    Some(0),
                    Some("output"),
                    Some(""),
                ))
                .unwrap();
            }
        }

        (manager, temp_dir, rt)
    }

    // Benchmark getting execution history
    for exec_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("get_execution_history", exec_count),
            exec_count,
            |b, &count| {
                b.iter_batched(
                    || setup_db_with_executions(count),
                    |(manager, _temp_dir, rt)| {
                        let history = rt
                            .block_on(
                                manager.get_execution_history(
                                    black_box("test_workflow"),
                                    black_box(10),
                                ),
                            )
                            .unwrap();
                        assert!(!history.is_empty());
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    // Benchmark getting task executions
    for exec_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("get_task_executions", exec_count),
            exec_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (manager, temp_dir, rt) = setup_db_with_executions(count);
                        let workflow_id = rt
                            .block_on(
                                manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")),
                            )
                            .unwrap();
                        let execution_id =
                            rt.block_on(manager.start_execution(workflow_id)).unwrap();
                        (manager, execution_id, temp_dir, rt)
                    },
                    |(manager, execution_id, _temp_dir, rt)| {
                        let _tasks = rt
                            .block_on(manager.get_task_executions(black_box(execution_id)))
                            .unwrap();
                        // May be empty if it's a new execution
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark workflow statistics calculation
fn bench_workflow_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_statistics");

    // Create databases with varying numbers of executions
    for exec_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(exec_count),
            exec_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (manager, temp_dir, rt) = create_temp_state_manager();
                        let workflow_id = rt
                            .block_on(
                                manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")),
                            )
                            .unwrap();

                        // Create executions with mixed success/failure
                        for i in 0..count {
                            let execution_id =
                                rt.block_on(manager.start_execution(workflow_id)).unwrap();
                            let status = if i % 3 == 0 {
                                TaskStatus::Failed
                            } else {
                                TaskStatus::Success
                            };
                            rt.block_on(manager.update_execution_status(execution_id, status))
                                .unwrap();
                        }

                        (manager, temp_dir, rt)
                    },
                    |(manager, _temp_dir, rt)| {
                        let stats = rt
                            .block_on(manager.get_workflow_statistics(black_box("test_workflow")))
                            .unwrap();
                        assert!(stats.total_executions > 0);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent database operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark concurrent writes (workflow executions)
    for concurrent_count in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_executions", concurrent_count),
            concurrent_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let (manager, temp_dir, rt) = create_temp_state_manager();
                        let workflow_id = rt
                            .block_on(
                                manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")),
                            )
                            .unwrap();
                        (manager, workflow_id, temp_dir, rt)
                    },
                    |(manager, workflow_id, _temp_dir, rt)| {
                        rt.block_on(async move {
                            let mut handles = vec![];
                            for _ in 0..count {
                                let mgr = manager.clone();
                                let wf_id = workflow_id;
                                handles.push(tokio::spawn(async move {
                                    let exec_id = mgr.start_execution(wf_id).await.unwrap();
                                    mgr.update_execution_status(exec_id, TaskStatus::Success)
                                        .await
                                        .unwrap();
                                }));
                            }
                            for handle in handles {
                                handle.await.unwrap();
                            }
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark database transaction overhead
fn bench_transaction_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("transaction_overhead");

    // Single insert
    group.bench_function("single_insert", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                (manager, workflow_id, temp_dir, rt)
            },
            |(manager, workflow_id, _temp_dir, rt)| {
                let execution_id = rt
                    .block_on(manager.start_execution(black_box(workflow_id)))
                    .unwrap();
                assert!(execution_id > 0);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Batch inserts (10 executions)
    group.bench_function("batch_10_inserts", |b| {
        b.iter_batched(
            || {
                let (manager, temp_dir, rt) = create_temp_state_manager();
                let workflow_id = rt
                    .block_on(manager.get_or_create_workflow("test_workflow", Some("0 2 * * *")))
                    .unwrap();
                (manager, workflow_id, temp_dir, rt)
            },
            |(manager, workflow_id, _temp_dir, rt)| {
                for _ in 0..10 {
                    let execution_id = rt
                        .block_on(manager.start_execution(black_box(workflow_id)))
                        .unwrap();
                    assert!(execution_id > 0);
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_workflow_creation,
    bench_execution_management,
    bench_task_execution_records,
    bench_execution_history_queries,
    bench_workflow_statistics,
    bench_concurrent_operations,
    bench_transaction_overhead
);
criterion_main!(benches);
