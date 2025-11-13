use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use picoflow::dag::DagEngine;
use picoflow::models::{ShellConfig, TaskConfig, TaskExecutorConfig, TaskType};

/// Create a linear chain of tasks: task0 -> task1 -> task2 -> ...
/// This represents the worst case for topological sort (maximum depth, no parallelism)
fn create_linear_chain(count: usize) -> Vec<TaskConfig> {
    let mut tasks = Vec::with_capacity(count);

    for i in 0..count {
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
            retry: Some(3),
            timeout: Some(300),
            continue_on_failure: false,
        });
    }

    tasks
}

/// Create a diamond-shaped DAG pattern: 1 -> 2,3,4,5 -> 1
/// This represents a common pattern with multiple parallel branches converging
fn create_diamond_dag(layers: usize) -> Vec<TaskConfig> {
    let mut tasks = Vec::new();
    let mut task_counter = 0;

    // Root task
    tasks.push(TaskConfig {
        name: format!("task{}", task_counter),
        task_type: TaskType::Shell,
        depends_on: vec![],
        config: TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/true".to_string(),
            args: vec![],
            workdir: None,
            env: None,
        }),
        retry: Some(3),
        timeout: Some(300),
        continue_on_failure: false,
    });
    task_counter += 1;

    // Create diamond layers
    for layer in 0..layers {
        let prev_root = if layer == 0 {
            "task0".to_string()
        } else {
            format!("task{}", task_counter - 1)
        };

        // Create 4 parallel branches
        let branch_start = task_counter;
        for _ in 0..4 {
            tasks.push(TaskConfig {
                name: format!("task{}", task_counter),
                task_type: TaskType::Shell,
                depends_on: vec![prev_root.clone()],
                config: TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/true".to_string(),
                    args: vec![],
                    workdir: None,
                    env: None,
                }),
                retry: Some(3),
                timeout: Some(300),
                continue_on_failure: false,
            });
            task_counter += 1;
        }

        // Converge to single task
        let deps: Vec<String> = (branch_start..task_counter)
            .map(|i| format!("task{}", i))
            .collect();
        tasks.push(TaskConfig {
            name: format!("task{}", task_counter),
            task_type: TaskType::Shell,
            depends_on: deps,
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/true".to_string(),
                args: vec![],
                workdir: None,
                env: None,
            }),
            retry: Some(3),
            timeout: Some(300),
            continue_on_failure: false,
        });
        task_counter += 1;
    }

    tasks
}

/// Create a wide parallel DAG: 1 root -> N parallel tasks -> 1 final task
/// This represents maximum parallelism scenario
fn create_wide_parallel(width: usize) -> Vec<TaskConfig> {
    let mut tasks = Vec::new();

    // Root task
    tasks.push(TaskConfig {
        name: "root".to_string(),
        task_type: TaskType::Shell,
        depends_on: vec![],
        config: TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/true".to_string(),
            args: vec![],
            workdir: None,
            env: None,
        }),
        retry: Some(3),
        timeout: Some(300),
        continue_on_failure: false,
    });

    // Parallel tasks
    for i in 0..width {
        tasks.push(TaskConfig {
            name: format!("parallel{}", i),
            task_type: TaskType::Shell,
            depends_on: vec!["root".to_string()],
            config: TaskExecutorConfig::Shell(ShellConfig {
                command: "/bin/true".to_string(),
                args: vec![],
                workdir: None,
                env: None,
            }),
            retry: Some(3),
            timeout: Some(300),
            continue_on_failure: false,
        });
    }

    // Final task that depends on all parallel tasks
    let parallel_deps: Vec<String> = (0..width).map(|i| format!("parallel{}", i)).collect();
    tasks.push(TaskConfig {
        name: "final".to_string(),
        task_type: TaskType::Shell,
        depends_on: parallel_deps,
        config: TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/true".to_string(),
            args: vec![],
            workdir: None,
            env: None,
        }),
        retry: Some(3),
        timeout: Some(300),
        continue_on_failure: false,
    });

    tasks
}

/// Benchmark DAG building and topological sort with various sizes
/// Target: <50ms for 100 tasks, <500ms for 1000 tasks (PRD PERF-005)
fn bench_dag_build_and_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_build_and_sort");

    for size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tasks = create_linear_chain(size);
            b.iter(|| {
                let dag = DagEngine::build(black_box(&tasks)).unwrap();
                let _sorted = dag.topological_sort().unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark cycle detection performance
fn bench_cycle_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("cycle_detection");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("acyclic", size),
            size,
            |b, &size| {
                let tasks = create_linear_chain(size);
                b.iter(|| {
                    let dag = DagEngine::build(black_box(&tasks)).unwrap();
                    dag.validate_acyclic().unwrap();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel level calculation for different DAG shapes
fn bench_parallel_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_levels");

    // Linear chain (worst case - no parallelism)
    for size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("linear", size),
            size,
            |b, &size| {
                let tasks = create_linear_chain(size);
                let dag = DagEngine::build(&tasks).unwrap();
                b.iter(|| {
                    let _levels = dag.parallel_levels();
                });
            },
        );
    }

    // Diamond pattern (moderate parallelism)
    for layers in [5, 10, 15].iter() {
        group.bench_with_input(
            BenchmarkId::new("diamond", layers),
            layers,
            |b, &layers| {
                let tasks = create_diamond_dag(layers);
                let dag = DagEngine::build(&tasks).unwrap();
                b.iter(|| {
                    let _levels = dag.parallel_levels();
                });
            },
        );
    }

    // Wide parallel (maximum parallelism)
    for width in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("wide_parallel", width),
            width,
            |b, &width| {
                let tasks = create_wide_parallel(width);
                let dag = DagEngine::build(&tasks).unwrap();
                b.iter(|| {
                    let _levels = dag.parallel_levels();
                });
            },
        );
    }

    group.finish();
}

/// Benchmark dependency queries
fn bench_dependency_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_queries");

    let tasks = create_diamond_dag(20);
    let dag = DagEngine::build(&tasks).unwrap();

    group.bench_function("get_dependencies", |b| {
        b.iter(|| {
            for i in 0..tasks.len() {
                let task_name = format!("task{}", i);
                let _deps = dag.get_dependencies(black_box(&task_name));
            }
        });
    });

    group.bench_function("get_dependents", |b| {
        b.iter(|| {
            for i in 0..tasks.len() {
                let task_name = format!("task{}", i);
                let _deps = dag.get_dependents(black_box(&task_name));
            }
        });
    });

    group.finish();
}

/// Benchmark topological sort only (without build)
fn bench_topological_sort_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("topological_sort_only");

    for size in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tasks = create_linear_chain(size);
            let dag = DagEngine::build(&tasks).unwrap();
            b.iter(|| {
                let _sorted = dag.topological_sort().unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dag_build_and_sort,
    bench_cycle_detection,
    bench_parallel_levels,
    bench_dependency_queries,
    bench_topological_sort_only
);
criterion_main!(benches);
