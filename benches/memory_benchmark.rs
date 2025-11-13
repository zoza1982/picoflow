use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use picoflow::dag::DagEngine;
use picoflow::models::{ShellConfig, TaskConfig, TaskExecutorConfig, TaskType};

/// Helper to create test tasks for memory benchmarking
fn create_test_tasks(count: usize) -> Vec<TaskConfig> {
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
            retry: Some(0),
            timeout: Some(30),
            continue_on_failure: false,
        });
    }
    tasks
}

/// Benchmark memory usage during DAG construction
/// This helps identify memory allocations in the DAG building process
fn bench_dag_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_memory_allocation");

    for size in [10, 50, 100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tasks = create_test_tasks(size);
            b.iter(|| {
                let dag = DagEngine::build(black_box(&tasks)).unwrap();
                // Force evaluation of all DAG operations
                let _sorted = dag.topological_sort().unwrap();
                let _levels = dag.parallel_levels();
                drop(dag);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dag_memory_allocation,
);
criterion_main!(benches);
