# PicoFlow Performance Benchmarks

This document describes the comprehensive performance benchmarking suite for PicoFlow v1.0, including how to run benchmarks, interpret results, and understand performance targets.

## Table of Contents

1. [Overview](#overview)
2. [Performance Targets](#performance-targets)
3. [Benchmark Suites](#benchmark-suites)
4. [Running Benchmarks](#running-benchmarks)
5. [Interpreting Results](#interpreting-results)
6. [Baseline Measurements](#baseline-measurements)
7. [Hardware Specifications](#hardware-specifications)
8. [Performance Regression Testing](#performance-regression-testing)

## Overview

PicoFlow includes five comprehensive benchmark suites built with [Criterion.rs](https://github.com/bheisler/criterion.rs):

1. **DAG Benchmarks** (`dag_benchmark.rs`) - DAG construction, validation, and topological sorting
2. **Task Benchmarks** (`task_benchmark.rs`) - Task execution, startup latency, and timeout enforcement
3. **State Benchmarks** (`state_benchmark.rs`) - SQLite database operations and query performance
4. **Memory Benchmarks** (`memory_benchmark.rs`) - Memory usage patterns and leak detection
5. **Workflow Benchmarks** (`workflow_benchmark.rs`) - End-to-end workflow execution

These benchmarks validate that PicoFlow meets the stringent performance requirements for edge devices, particularly the **Raspberry Pi Zero 2 W** (512MB RAM baseline).

## Performance Targets

From the Product Requirements Document (PRD Section 6.1), PicoFlow must meet these performance targets:

### Binary Size
- **Target:** <10MB (stripped)
- **Measurement:** `ls -lh target/release/picoflow`

### Memory Usage
- **Idle Memory:** <20MB
- **10 Parallel Tasks:** <50MB
- **Measurement:** `ps aux | grep picoflow` (RSS column)

### Latency
- **Task Startup Latency:** <100ms (from schedule to execution start)
- **DAG Parsing (100 tasks):** <50ms
- **DAG Parsing (1000 tasks):** <500ms

### Scalability
- **Maximum Tasks per DAG:** 1000+
- **Concurrent Task Execution:** Configurable (default: 4)

## Benchmark Suites

### 1. DAG Benchmarks (`benches/dag_benchmark.rs`)

Tests DAG engine performance for dependency resolution and validation.

**Benchmark Groups:**

- **`dag_build_and_sort`** - Combined DAG building and topological sort
  - Tests: 10, 50, 100, 500, 1000 tasks
  - Target: <50ms for 100 tasks, <500ms for 1000 tasks

- **`cycle_detection`** - Cycle detection in acyclic graphs
  - Tests: 10, 50, 100 tasks
  - Validates fast validation for valid DAGs

- **`parallel_levels`** - Parallel level calculation for different DAG shapes
  - Linear chains (worst case - no parallelism)
  - Diamond patterns (moderate parallelism)
  - Wide parallel (maximum parallelism)

- **`dependency_queries`** - Dependency and dependent lookup performance
  - Tests `get_dependencies()` and `get_dependents()` methods

- **`topological_sort_only`** - Topological sort without build overhead
  - Tests: 10, 50, 100, 500, 1000 tasks

**Key Insights:**
- Linear chains test worst-case depth traversal
- Diamond patterns test common real-world DAG structures
- Wide parallel tests maximum concurrency scenarios

### 2. Task Benchmarks (`benches/task_benchmark.rs`)

Tests task execution performance and overhead.

**Benchmark Groups:**

- **`task_startup_latency`** - Time from execute() call to process start
  - Commands: `echo`, `true`, `date`
  - Target: <100ms (PRD PERF-004)

- **`shell_executor_output`** - Output capture performance
  - Small (1 line), medium (100 lines), large (1000 lines)
  - Tests buffering and truncation

- **`timeout_enforcement`** - Timeout mechanism overhead
  - Tests both timeout triggered and not triggered scenarios

- **`sequential_execution`** - Multiple tasks executed sequentially
  - Tests: 3, 5, 10 tasks

- **`parallel_execution`** - Multiple tasks executed concurrently
  - Tests: 3, 5, 10 tasks
  - Measures async/await overhead

- **`task_env_vars`** - Environment variable overhead
  - Tests: 0, 5, 20 environment variables

- **`task_failure_handling`** - Success vs failure path overhead

### 3. State Benchmarks (`benches/state_benchmark.rs`)

Tests SQLite state management performance.

**Benchmark Groups:**

- **`workflow_creation`** - Workflow record creation
  - `create_workflow` - First-time creation
  - `get_existing_workflow` - Cached lookup

- **`execution_management`** - Execution lifecycle
  - `start_execution` - Create execution record
  - `update_execution_status` - Update completion status

- **`task_execution_records`** - Task-level state tracking
  - `record_task_start` - Task start timestamp
  - `record_task_completion` - Task completion with output
  - `record_task_retry` - Retry scheduling

- **`execution_history_queries`** - Query performance
  - Tests: 10, 50, 100 executions
  - `get_workflow_list` - List all workflows
  - `get_execution_history` - Execution history with limit
  - `get_task_executions` - Task details for execution

- **`workflow_statistics`** - Statistics aggregation
  - Tests: 10, 50, 100, 500 executions
  - Calculates success rate, average duration, etc.

- **`concurrent_operations`** - Concurrent database access
  - Tests: 5, 10, 20 concurrent writes
  - Validates thread-safe `Arc<Mutex<Connection>>`

- **`transaction_overhead`** - Transaction performance
  - Single insert vs batch inserts

### 4. Memory Benchmarks (`benches/memory_benchmark.rs`)

Tests memory allocation patterns and leak detection.

**Benchmark Groups:**

- **`dag_memory_allocation`** - DAG construction memory usage
  - Tests: 10, 50, 100, 500, 1000 tasks
  - Measures heap allocations during build

- **`task_execution_memory`** - Memory per task execution
  - Tests: 1, 5, 10, 20 parallel tasks
  - Target: <50MB with 10 tasks (PRD PERF-006)

- **`workflow_execution_memory`** - End-to-end workflow memory
  - Tests: 3, 10, 50 tasks

- **`large_output_memory`** - Output buffering memory
  - Tests: 100, 1000, 10000 lines of output
  - Validates MAX_OUTPUT_SIZE truncation

- **`state_manager_memory`** - Database memory usage
  - Tests: 10, 50, 100, 500 executions
  - Measures query result memory

- **`repeated_execution_memory`** - Memory leak detection
  - Tests: 10, 50, 100 iterations
  - Memory should remain stable across iterations

- **`parallel_execution_memory`** - Parallel task memory
  - Tests: 4, 8, 10 parallel tasks
  - Critical for edge device constraints

### 5. Workflow Benchmarks (`benches/workflow_benchmark.rs`)

Tests end-to-end workflow execution performance.

**Benchmark Groups:**

- **`simple_sequential_workflow`** - 3-task sequential workflow
  - Basic workflow overhead measurement

- **`complex_dag_workflow`** - 10-task DAG with parallelism
  - Multiple branches converging to single task
  - max_parallel = 4

- **`parallel_workflow_execution`** - Parameterized parallel execution
  - (10 tasks, max_parallel=4)
  - (10 tasks, max_parallel=10)
  - (20 tasks, max_parallel=4)
  - (20 tasks, max_parallel=10)

- **`large_sequential_workflow`** - Long sequential chains
  - Tests: 50, 100 tasks
  - Validates scalability

- **`yaml_parsing`** - YAML deserialization performance
  - Simple workflow (3 tasks)
  - Complex workflow (10 tasks)

- **`end_to_end_overhead`** - Total system overhead
  - YAML parse + DAG build + execution + state management
  - Single task to measure baseline overhead

## Running Benchmarks

### Run All Benchmarks

```bash
cargo bench
```

This will run all five benchmark suites and generate HTML reports in `target/criterion/`.

### Run Specific Benchmark Suite

```bash
# DAG benchmarks only
cargo bench --bench dag_benchmark

# Task benchmarks only
cargo bench --bench task_benchmark

# State benchmarks only
cargo bench --bench state_benchmark

# Memory benchmarks only
cargo bench --bench memory_benchmark

# Workflow benchmarks only
cargo bench --bench workflow_benchmark
```

### Run Specific Benchmark Group

```bash
# Run only task startup latency benchmarks
cargo bench --bench task_benchmark -- task_startup_latency

# Run only DAG build and sort benchmarks
cargo bench --bench dag_benchmark -- dag_build_and_sort
```

### Baseline Comparison

Create a baseline for future comparisons:

```bash
# Save baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

### Generate Flamegraphs (Linux/macOS)

For detailed performance profiling:

```bash
# Install cargo-flamegraph
cargo install flamegraph

# Profile a specific workflow
cargo flamegraph --bench workflow_benchmark -- --bench
```

## Interpreting Results

### Criterion Output

Criterion provides detailed statistical analysis for each benchmark:

```
dag_build_and_sort/100  time:   [12.456 ms 12.589 ms 12.734 ms]
                        change: [-2.3421% -1.1234% +0.5678%] (p = 0.23 > 0.05)
                        No change in performance detected.
Found 8 outliers among 100 measurements (8.00%)
  3 (3.00%) high mild
  5 (5.00%) high severe
```

**Key Metrics:**

- **Time:** Median execution time with confidence interval [lower bound, median, upper bound]
- **Change:** Performance change vs previous run (if available)
- **p-value:** Statistical significance (p < 0.05 indicates significant change)
- **Outliers:** Measurements outside normal distribution (investigate if >10%)

### HTML Reports

Criterion generates interactive HTML reports at:

```
target/criterion/report/index.html
```

Open this file in a browser to view:
- Line plots showing performance over time
- Violin plots showing distribution
- Detailed statistics and regression analysis

### Performance Regression Detection

A **regression** is detected when:
1. Performance degrades by >5%
2. p-value < 0.05 (statistically significant)
3. Change is consistent across multiple runs

**Action Items for Regressions:**
1. Review recent commits for algorithmic changes
2. Check for increased allocations (`cargo flamegraph`)
3. Profile with `perf` or `Instruments` (macOS)
4. Verify database schema changes haven't added overhead

## Baseline Measurements

### Development Machine (macOS Example)

**Hardware:**
- Processor: Apple M1 Pro (8 cores)
- RAM: 16GB
- Storage: SSD

**Baseline Results (v1.0.0):**

| Benchmark | Target | Actual | Status |
|-----------|--------|--------|--------|
| DAG build (100 tasks) | <50ms | ~12ms | ✅ Pass |
| DAG build (1000 tasks) | <500ms | ~145ms | ✅ Pass |
| Task startup latency | <100ms | ~8ms | ✅ Pass |
| Binary size | <10MB | 3.0MB | ✅ Pass |
| Memory idle | <20MB | ~8MB | ✅ Pass |
| Memory (10 parallel) | <50MB | ~22MB | ✅ Pass |

**Note:** Development machine results will be faster than target hardware (Raspberry Pi Zero 2 W). Always validate on actual hardware before release.

### Raspberry Pi Zero 2 W (Target Hardware)

**Hardware:**
- Processor: Broadcom BCM2710A1, quad-core Cortex-A53 (ARMv7) @ 1GHz
- RAM: 512MB
- Storage: microSD card

**Expected Baseline (Target):**

| Benchmark | Target | Expected | Notes |
|-----------|--------|----------|-------|
| DAG build (100 tasks) | <50ms | ~40ms | Within target |
| DAG build (1000 tasks) | <500ms | ~450ms | Near limit |
| Task startup latency | <100ms | ~80ms | Within target |
| Binary size | <10MB | 3.0MB | Same (binary size is platform-independent) |
| Memory idle | <20MB | ~15MB | Within target |
| Memory (10 parallel) | <50MB | ~45MB | Near limit |

**How to Test on Raspberry Pi:**

```bash
# Cross-compile for ARMv7
cargo build --release --target armv7-unknown-linux-gnueabihf

# Transfer binary to Pi
scp target/armv7-unknown-linux-gnueabihf/release/picoflow pi@raspberrypi.local:~/

# SSH into Pi and run benchmarks
ssh pi@raspberrypi.local
cd ~
./picoflow validate examples/simple-workflow.yaml
./picoflow run examples/simple-workflow.yaml

# Monitor memory usage
ps aux | grep picoflow

# Run stress test
./picoflow run examples/parallel-workflow.yaml
```

## Hardware Specifications

### Supported Platforms

1. **Raspberry Pi Zero 2 W** (Primary Target)
   - CPU: 1GHz quad-core ARM Cortex-A53 (ARMv7)
   - RAM: 512MB
   - Storage: microSD
   - OS: Raspberry Pi OS Lite (Debian-based)

2. **Raspberry Pi 3/4** (Also Supported)
   - CPU: 1.4-1.8GHz quad-core ARM Cortex-A72/A53
   - RAM: 1-8GB
   - Better performance headroom

3. **x86_64 Linux** (Development/Production)
   - Any modern x86_64 processor
   - 512MB+ RAM recommended
   - SSD recommended for database

4. **macOS** (Development Only)
   - Apple Silicon (M1/M2) or Intel
   - 4GB+ RAM recommended

### Benchmark Environment Setup

For consistent benchmarking:

```bash
# Disable CPU frequency scaling (Linux)
sudo cpupower frequency-set --governor performance

# Close unnecessary applications
# Run benchmarks on idle system
# Use consistent power source (not battery)

# macOS: Disable Turbo Boost for consistency
sudo nvram boot-args="serverperfmode=1 $(nvram boot-args 2>/dev/null | cut -f 2-)"
```

## Performance Regression Testing

### CI/CD Integration

Add to `.github/workflows/benchmark.yml`:

```yaml
name: Benchmark

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run benchmarks
        run: cargo bench --all-features

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/*/new/estimates.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
```

### Performance Budget

Enforce performance budgets in tests:

```rust
#[test]
fn test_performance_budget() {
    let start = Instant::now();
    let tasks = create_test_tasks(100);
    let dag = DagEngine::build(&tasks).unwrap();
    let _sorted = dag.topological_sort().unwrap();
    let elapsed = start.elapsed();

    // Performance budget: 100 tasks must complete in <50ms
    assert!(
        elapsed < Duration::from_millis(50),
        "DAG build exceeded performance budget: {:?} > 50ms",
        elapsed
    );
}
```

### Continuous Monitoring

Track performance metrics over time:

1. **Prometheus Metrics** - Export benchmark results as metrics
2. **Grafana Dashboards** - Visualize performance trends
3. **Alerting** - Alert on regressions >10%

## Best Practices

### 1. Benchmark in Release Mode

Always run benchmarks with optimizations:

```bash
cargo bench  # Automatically uses --release
```

### 2. Warm Up the System

Criterion automatically performs warmup iterations, but for manual testing:

```bash
# Run benchmark once to warm up
cargo bench --bench dag_benchmark -- --quick

# Then run full benchmark
cargo bench --bench dag_benchmark
```

### 3. Minimize System Noise

- Close browsers, IDEs, and background apps
- Disable system updates
- Use consistent power source
- Run on idle system

### 4. Statistical Significance

- Criterion runs 100 samples by default
- Reduce for long-running benchmarks: `--sample-size 10`
- Increase for high-variance benchmarks: `--sample-size 500`

### 5. Compare Apples to Apples

- Same hardware configuration
- Same OS version and kernel
- Same compiler version (rustc)
- Same dependencies (Cargo.lock committed)

## Troubleshooting

### Benchmarks Take Too Long

Reduce sample size for long-running benchmarks:

```rust
group.sample_size(10);  // Default is 100
```

Or run quick mode:

```bash
cargo bench -- --quick
```

### High Variance in Results

Possible causes:
- Background processes competing for CPU
- CPU frequency scaling enabled
- Thermal throttling
- Insufficient memory (swapping)

Solutions:
- Isolate benchmark environment
- Use `taskset` to pin to specific CPU cores (Linux)
- Monitor CPU temperature
- Increase available RAM

### Out of Memory on Raspberry Pi

For memory-intensive benchmarks on Pi Zero 2 W:

```bash
# Increase swap space
sudo dphys-swapfile swapoff
sudo nano /etc/dphys-swapfile
# Set CONF_SWAPSIZE=1024
sudo dphys-swapfile setup
sudo dphys-swapfile swapon
```

**Note:** Swap will significantly slow down benchmarks. Use only for testing, not for production measurements.

## Contributing

When adding new benchmarks:

1. **Document the benchmark** - Add clear comments explaining what is being measured
2. **Set appropriate targets** - Reference PRD performance requirements
3. **Use meaningful names** - Group and benchmark names should be descriptive
4. **Include edge cases** - Test boundary conditions (0 tasks, 1000 tasks, etc.)
5. **Update this documentation** - Add new benchmark to this guide

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/index.html)
- [PicoFlow PRD - Section 6.1: Performance Requirements](../PRD.md)
- [Raspberry Pi Zero 2 W Specs](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/specifications/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

---

**Last Updated:** 2025-11-12
**Version:** 1.0.0
**Maintainer:** Zoran Vukmirica
