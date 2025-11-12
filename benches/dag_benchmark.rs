use criterion::{black_box, criterion_group, criterion_main, Criterion};
use picoflow::dag::DagEngine;
use picoflow::models::{ShellConfig, TaskConfig, TaskExecutorConfig, TaskType};

fn create_test_tasks(count: usize) -> Vec<TaskConfig> {
    let mut tasks = Vec::with_capacity(count);

    // Create linear chain: task0 -> task1 -> task2 -> ...
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

fn bench_dag_build_and_sort(c: &mut Criterion) {
    c.bench_function("dag_build_100_tasks", |b| {
        let tasks = create_test_tasks(100);
        b.iter(|| {
            let dag = DagEngine::build(black_box(&tasks)).unwrap();
            let _sorted = dag.topological_sort().unwrap();
        });
    });

    c.bench_function("dag_build_1000_tasks", |b| {
        let tasks = create_test_tasks(1000);
        b.iter(|| {
            let dag = DagEngine::build(black_box(&tasks)).unwrap();
            let _sorted = dag.topological_sort().unwrap();
        });
    });
}

criterion_group!(benches, bench_dag_build_and_sort);
criterion_main!(benches);
