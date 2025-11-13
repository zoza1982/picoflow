use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use picoflow::executors::shell::ShellExecutor;
use picoflow::executors::ExecutorTrait;
use picoflow::models::{ShellConfig, TaskExecutorConfig};
use tokio::runtime::Runtime;

/// Create a shell executor and task configuration for benchmarking
fn create_shell_config(command: &str, args: Vec<String>) -> (ShellExecutor, TaskExecutorConfig) {
    let executor = ShellExecutor::new();
    let config = TaskExecutorConfig::Shell(ShellConfig {
        command: command.to_string(),
        args,
        workdir: None,
        env: None,
    });
    (executor, config)
}

/// Benchmark task startup latency (time from execute call to process start)
/// Target: <100ms (PRD PERF-004)
fn bench_task_startup_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("task_startup_latency");

    // Fast command (echo)
    group.bench_function("echo_hello", |b| {
        let (executor, config) = create_shell_config("/bin/echo", vec!["hello".to_string()]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // No-op command (true)
    group.bench_function("true_command", |b| {
        let (executor, config) = create_shell_config("/bin/true", vec![]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // Date command
    group.bench_function("date_command", |b| {
        let (executor, config) = create_shell_config("/bin/date", vec![]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    group.finish();
}

/// Benchmark shell executor with different output sizes
fn bench_shell_executor_output_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("shell_executor_output");

    // Small output (1 line)
    group.bench_function("small_output_1_line", |b| {
        let (executor, config) = create_shell_config("/bin/echo", vec!["hello".to_string()]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.stdout.is_some());
            })
        });
    });

    // Medium output (100 lines)
    group.bench_function("medium_output_100_lines", |b| {
        let (executor, config) = create_shell_config(
            "/bin/sh",
            vec![
                "-c".to_string(),
                "for i in $(seq 1 100); do echo line $i; done".to_string(),
            ],
        );
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.stdout.is_some());
            })
        });
    });

    // Large output (1000 lines)
    group.bench_function("large_output_1000_lines", |b| {
        let (executor, config) = create_shell_config(
            "/bin/sh",
            vec![
                "-c".to_string(),
                "for i in $(seq 1 1000); do echo line $i; done".to_string(),
            ],
        );
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.stdout.is_some());
            })
        });
    });

    group.finish();
}

/// Benchmark timeout enforcement overhead
fn bench_timeout_enforcement(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("timeout_enforcement");

    // Task that completes before timeout (no timeout triggered)
    group.bench_function("no_timeout_triggered", |b| {
        let executor = ShellExecutor::new();
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/sleep".to_string(),
            args: vec!["0.1".to_string()],
            workdir: None,
            env: None,
        });
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // Note: Timeout is enforced at the scheduler level, not executor level
    // So we can't easily test timeout in these benchmarks without the full scheduler

    group.finish();
}

/// Benchmark sequential execution overhead (multiple tasks in sequence)
fn bench_sequential_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sequential_execution");

    for task_count in [3, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(task_count),
            task_count,
            |b, &count| {
                let executor = ShellExecutor::new();
                let config = TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/true".to_string(),
                    args: vec![],
                    workdir: None,
                    env: None,
                });

                b.iter(|| {
                    rt.block_on(async {
                        for _ in 0..count {
                            let result = executor.execute(black_box(&config)).await.unwrap();
                            assert!(result.status == picoflow::models::TaskStatus::Success);
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark parallel execution overhead (multiple tasks concurrently)
fn bench_parallel_execution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("parallel_execution");

    for task_count in [3, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(task_count),
            task_count,
            |b, &count| {
                let executor = ShellExecutor::new();
                let config = TaskExecutorConfig::Shell(ShellConfig {
                    command: "/bin/true".to_string(),
                    args: vec![],
                    workdir: None,
                    env: None,
                });

                b.iter(|| {
                    rt.block_on(async {
                        let futures: Vec<_> = (0..count)
                            .map(|_| executor.execute(black_box(&config)))
                            .collect();
                        let results = futures::future::join_all(futures).await;
                        for result in results {
                            assert!(result.is_ok());
                            assert!(result.unwrap().status == picoflow::models::TaskStatus::Success);
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark task with environment variables
fn bench_task_with_env_vars(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("task_env_vars");

    // No environment variables
    group.bench_function("no_env", |b| {
        let (executor, config) = create_shell_config("/bin/echo", vec!["hello".to_string()]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // With 5 environment variables
    group.bench_function("with_5_env", |b| {
        let executor = ShellExecutor::new();
        let mut env = std::collections::HashMap::new();
        for i in 0..5 {
            env.insert(format!("VAR{}", i), format!("value{}", i));
        }
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            workdir: None,
            env: Some(env),
        });
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // With 20 environment variables
    group.bench_function("with_20_env", |b| {
        let executor = ShellExecutor::new();
        let mut env = std::collections::HashMap::new();
        for i in 0..20 {
            env.insert(format!("VAR{}", i), format!("value{}", i));
        }
        let config = TaskExecutorConfig::Shell(ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            workdir: None,
            env: Some(env),
        });
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    group.finish();
}

/// Benchmark task failure handling
fn bench_task_failure_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("task_failure");

    // Successful task
    group.bench_function("success", |b| {
        let (executor, config) = create_shell_config("/bin/true", vec![]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Success);
            })
        });
    });

    // Failed task (exit code 1)
    group.bench_function("failure", |b| {
        let (executor, config) = create_shell_config("/bin/false", vec![]);
        b.iter(|| {
            rt.block_on(async {
                let result = executor.execute(black_box(&config)).await.unwrap();
                assert!(result.status == picoflow::models::TaskStatus::Failed);
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_task_startup_latency,
    bench_shell_executor_output_sizes,
    bench_timeout_enforcement,
    bench_sequential_execution,
    bench_parallel_execution,
    bench_task_with_env_vars,
    bench_task_failure_handling
);
criterion_main!(benches);
