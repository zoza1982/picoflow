use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use picoflow::models::{
    ShellConfig, TaskConfig, TaskExecutorConfig, TaskType, WorkflowConfig, WorkflowGlobalConfig,
};
use picoflow::scheduler::TaskScheduler;
use picoflow::state::StateManager;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Create a simple sequential workflow (3 tasks in a chain)
fn create_simple_sequential_workflow() -> WorkflowConfig {
    WorkflowConfig {
        name: "simple_sequential".to_string(),
        description: Some("Simple 3-task sequential workflow".to_string()),
        schedule: None,
        config: WorkflowGlobalConfig {
            max_parallel: 1,
            retry_default: 0,
            timeout_default: 30,
        },
        tasks: vec![
            TaskConfig {
                name: "task1".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Task 1".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "task2".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["task1".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Task 2".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "task3".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["task2".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Task 3".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
        ],
    }
}

/// Create a complex DAG workflow (10 tasks with dependencies)
fn create_complex_dag_workflow() -> WorkflowConfig {
    WorkflowConfig {
        name: "complex_dag".to_string(),
        description: Some("Complex 10-task DAG workflow".to_string()),
        schedule: None,
        config: WorkflowGlobalConfig {
            max_parallel: 4,
            retry_default: 0,
            timeout_default: 30,
        },
        tasks: vec![
            // Root task
            TaskConfig {
                name: "init".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Initialize".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            // Parallel branch 1
            TaskConfig {
                name: "process_a1".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["init".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process A1".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "process_a2".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["process_a1".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process A2".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            // Parallel branch 2
            TaskConfig {
                name: "process_b1".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["init".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process B1".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "process_b2".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["process_b1".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process B2".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            // Parallel branch 3
            TaskConfig {
                name: "process_c1".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["init".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process C1".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "process_c2".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["process_c1".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Process C2".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            // Convergence task
            TaskConfig {
                name: "aggregate".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec![
                    "process_a2".to_string(),
                    "process_b2".to_string(),
                    "process_c2".to_string(),
                ],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Aggregate".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            // Final tasks
            TaskConfig {
                name: "validate".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["aggregate".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Validate".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
            TaskConfig {
                name: "finalize".to_string(),
                task_type: TaskType::Shell,
                depends_on: vec!["validate".to_string()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/echo".to_string(),
                    args: vec!["Finalize".to_string()],
                    workdir: None,
                    env: None,
                }),
                retry: Some(0),
                timeout: Some(30),
                continue_on_failure: false,
            },
        ],
    }
}

/// Create a parallel workflow (N tasks that can run concurrently)
fn create_parallel_workflow(task_count: usize, max_parallel: usize) -> WorkflowConfig {
    let mut tasks = vec![TaskConfig {
        name: "start".to_string(),
        task_type: TaskType::Shell,
        depends_on: vec![],
        config: TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["Start".to_string()],
            workdir: None,
            env: None,
        }),
        retry: Some(0),
        timeout: Some(30),
        continue_on_failure: false,
    }];

    for i in 0..task_count {
        tasks.push(TaskConfig {
            name: format!("parallel{}", i),
            task_type: TaskType::Shell,
            depends_on: vec!["start".to_string()],
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/echo".to_string(),
                args: vec![format!("Parallel task {}", i)],
                workdir: None,
                env: None,
            }),
            retry: Some(0),
            timeout: Some(30),
            continue_on_failure: false,
        });
    }

    let parallel_deps: Vec<String> = (0..task_count).map(|i| format!("parallel{}", i)).collect();
    tasks.push(TaskConfig {
        name: "finish".to_string(),
        task_type: TaskType::Shell,
        depends_on: parallel_deps,
        config: TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["Finish".to_string()],
            workdir: None,
            env: None,
        }),
        retry: Some(0),
        timeout: Some(30),
        continue_on_failure: false,
    });

    WorkflowConfig {
        name: format!("parallel_{}_tasks", task_count),
        description: Some(format!("{} parallel tasks", task_count)),
        schedule: None,
        config: WorkflowGlobalConfig {
            max_parallel,
            retry_default: 0,
            timeout_default: 30,
        },
        tasks,
    }
}

/// Create a large sequential workflow (N tasks in a chain)
fn create_large_sequential_workflow(task_count: usize) -> WorkflowConfig {
    let mut tasks = Vec::with_capacity(task_count);

    for i in 0..task_count {
        let depends_on = if i == 0 {
            vec![]
        } else {
            vec![format!("task{}", i - 1)]
        };

        tasks.push(TaskConfig {
            name: format!("task{}", i),
            task_type: TaskType::Shell,
            depends_on,
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/true".to_string(),
                args: vec![],
                workdir: None,
                env: None,
            }),
            retry: Some(0),
            timeout: Some(30),
            continue_on_failure: false,
        });
    }

    WorkflowConfig {
        name: format!("sequential_{}_tasks", task_count),
        description: Some(format!("{} sequential tasks", task_count)),
        schedule: None,
        config: WorkflowGlobalConfig {
            max_parallel: 1,
            retry_default: 0,
            timeout_default: 30,
        },
        tasks,
    }
}

/// Benchmark simple sequential workflow execution
fn bench_simple_sequential_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("simple_sequential_workflow");

    group.bench_function("3_tasks", |b| {
        b.iter_batched(
            || {
                let workflow = create_simple_sequential_workflow();
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("bench.db");
                let state_manager = Arc::new(StateManager::new(&db_path).unwrap());
                let scheduler = TaskScheduler::new(state_manager);
                (scheduler, workflow, temp_dir)
            },
            |(scheduler, workflow, _temp_dir)| {
                rt.block_on(async move {
                    let success = scheduler
                        .execute_workflow(black_box(&workflow))
                        .await
                        .unwrap();
                    assert!(success);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark complex DAG workflow execution
fn bench_complex_dag_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("complex_dag_workflow");

    group.bench_function("10_tasks_parallel", |b| {
        b.iter_batched(
            || {
                let workflow = create_complex_dag_workflow();
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("bench.db");
                let state_manager = Arc::new(StateManager::new(&db_path).unwrap());
                let scheduler = TaskScheduler::new(state_manager);
                (scheduler, workflow, temp_dir)
            },
            |(scheduler, workflow, _temp_dir)| {
                rt.block_on(async move {
                    let success = scheduler
                        .execute_workflow(black_box(&workflow))
                        .await
                        .unwrap();
                    assert!(success);
                })
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark parallel workflow execution with different concurrency levels
fn bench_parallel_workflow_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("parallel_workflow_execution");

    for (task_count, max_parallel) in [(10, 4), (10, 10), (20, 4), (20, 10)].iter() {
        group.throughput(Throughput::Elements(*task_count as u64));
        group.bench_with_input(
            BenchmarkId::new(
                format!("tasks_{}_parallel_{}", task_count, max_parallel),
                task_count,
            ),
            &(*task_count, *max_parallel),
            |b, &(tasks, parallel)| {
                b.iter_batched(
                    || {
                        let workflow = create_parallel_workflow(tasks, parallel);
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let state_manager = Arc::new(StateManager::new(&db_path).unwrap());
                        let scheduler = TaskScheduler::new(state_manager);
                        (scheduler, workflow, temp_dir)
                    },
                    |(scheduler, workflow, _temp_dir)| {
                        rt.block_on(async move {
                            let success = scheduler
                                .execute_workflow(black_box(&workflow))
                                .await
                                .unwrap();
                            assert!(success);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark large sequential workflow (100 tasks)
/// This tests scalability for workflows with many tasks
fn bench_large_sequential_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("large_sequential_workflow");
    group.sample_size(10); // Reduce sample size for long-running benchmarks

    for task_count in [50, 100].iter() {
        group.throughput(Throughput::Elements(*task_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(task_count),
            task_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let workflow = create_large_sequential_workflow(count);
                        let temp_dir = TempDir::new().unwrap();
                        let db_path = temp_dir.path().join("bench.db");
                        let state_manager = Arc::new(StateManager::new(&db_path).unwrap());
                        let scheduler = TaskScheduler::new(state_manager);
                        (scheduler, workflow, temp_dir)
                    },
                    |(scheduler, workflow, _temp_dir)| {
                        rt.block_on(async move {
                            let success = scheduler
                                .execute_workflow(black_box(&workflow))
                                .await
                                .unwrap();
                            assert!(success);
                        })
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark YAML parsing and workflow loading
fn bench_yaml_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_parsing");

    // Simple workflow YAML
    let simple_yaml = r#"
name: simple_workflow
description: "A simple test workflow"
schedule: "0 2 * * *"

config:
  max_parallel: 1
  retry_default: 3
  timeout_default: 300

tasks:
  - name: task1
    type: shell
    config:
      command: "/bin/echo"
      args: ["hello"]
  - name: task2
    type: shell
    depends_on: [task1]
    config:
      command: "/bin/echo"
      args: ["world"]
  - name: task3
    type: shell
    depends_on: [task2]
    config:
      command: "/bin/echo"
      args: ["done"]
"#;

    group.bench_function("parse_simple_workflow", |b| {
        b.iter(|| {
            let _workflow: WorkflowConfig = serde_yaml::from_str(black_box(simple_yaml)).unwrap();
        });
    });

    // Complex workflow YAML (10 tasks)
    let complex_yaml = r#"
name: complex_workflow
description: "A complex test workflow"
schedule: "0 2 * * *"

config:
  max_parallel: 4
  retry_default: 3
  timeout_default: 300

tasks:
  - name: init
    type: shell
    config:
      command: "/bin/echo"
      args: ["init"]
  - name: process_a1
    type: shell
    depends_on: [init]
    config:
      command: "/bin/echo"
      args: ["a1"]
  - name: process_a2
    type: shell
    depends_on: [process_a1]
    config:
      command: "/bin/echo"
      args: ["a2"]
  - name: process_b1
    type: shell
    depends_on: [init]
    config:
      command: "/bin/echo"
      args: ["b1"]
  - name: process_b2
    type: shell
    depends_on: [process_b1]
    config:
      command: "/bin/echo"
      args: ["b2"]
  - name: process_c1
    type: shell
    depends_on: [init]
    config:
      command: "/bin/echo"
      args: ["c1"]
  - name: process_c2
    type: shell
    depends_on: [process_c1]
    config:
      command: "/bin/echo"
      args: ["c2"]
  - name: aggregate
    type: shell
    depends_on: [process_a2, process_b2, process_c2]
    config:
      command: "/bin/echo"
      args: ["aggregate"]
  - name: validate
    type: shell
    depends_on: [aggregate]
    config:
      command: "/bin/echo"
      args: ["validate"]
  - name: finalize
    type: shell
    depends_on: [validate]
    config:
      command: "/bin/echo"
      args: ["finalize"]
"#;

    group.bench_function("parse_complex_workflow", |b| {
        b.iter(|| {
            let _workflow: WorkflowConfig = serde_yaml::from_str(black_box(complex_yaml)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark end-to-end workflow execution including all overhead
fn bench_end_to_end_overhead(c: &mut Criterion) {
    let _rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end_overhead");

    // Measure total overhead: YAML parse + DAG build + execution + state management
    let workflow_yaml = r#"
name: overhead_test
config:
  max_parallel: 1

tasks:
  - name: single_task
    type: shell
    config:
      command: "/bin/true"
"#;

    group.bench_function("single_task_total_overhead", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("bench.db");
                let state_manager = Arc::new(StateManager::new(&db_path).unwrap());
                let scheduler = TaskScheduler::new(state_manager);
                (scheduler, temp_dir)
            },
            |(scheduler, _temp_dir)| async move {
                // Parse YAML
                let workflow: WorkflowConfig =
                    serde_yaml::from_str(black_box(workflow_yaml)).unwrap();
                // Execute workflow
                let success = scheduler.execute_workflow(&workflow).await.unwrap();
                assert!(success);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_sequential_workflow,
    bench_complex_dag_workflow,
    bench_parallel_workflow_execution,
    bench_large_sequential_workflow,
    bench_yaml_parsing,
    bench_end_to_end_overhead
);
criterion_main!(benches);
