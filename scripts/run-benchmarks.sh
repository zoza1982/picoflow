#!/usr/bin/env bash
#
# run-benchmarks.sh - Run all PicoFlow benchmarks and display results
#
# This script runs all Criterion benchmarks and displays formatted results.
# Benchmarks cover:
# - DAG parsing and topological sort (target: <50ms for 100 tasks)
# - Task execution startup latency (target: <100ms)
# - State machine transitions
# - Workflow parsing and validation
# - Memory usage patterns
#
# Usage:
#   ./scripts/run-benchmarks.sh [OPTIONS]
#
# Options:
#   --quick      Run quick benchmarks (fewer samples)
#   --baseline   Save results as baseline for comparison
#   --compare    Compare against saved baseline
#   --verbose    Show full benchmark output
#   --help       Show this help message
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
QUICK_MODE=false
BASELINE_MODE=false
COMPARE_MODE=false
VERBOSE=false
BENCHMARK_FLAGS=""

# Benchmarks to run
BENCHMARKS=(
    "dag_benchmark"
    "task_benchmark"
    "state_benchmark"
    "workflow_benchmark"
    "memory_benchmark"
)

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --quick)
                QUICK_MODE=true
                BENCHMARK_FLAGS="--quick"
                shift
                ;;
            --baseline)
                BASELINE_MODE=true
                BENCHMARK_FLAGS="--save-baseline main"
                shift
                ;;
            --compare)
                COMPARE_MODE=true
                BENCHMARK_FLAGS="--baseline main"
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --help)
                sed -n '2,18p' "$0" | sed 's/^# //'
                exit 0
                ;;
            *)
                echo -e "${RED}Error: Unknown option: $1${NC}"
                echo "Run with --help for usage information"
                exit 1
                ;;
        esac
    done
}

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_benchmark() {
    echo -e "${CYAN}[BENCHMARK]${NC} $1"
}

# Extract key metrics from benchmark output
extract_metrics() {
    local bench_name="$1"
    local output="$2"

    echo ""
    echo -e "${BOLD}${CYAN}=== $bench_name ===${NC}"
    echo ""

    # Extract time measurements (format: "benchmark_name   time:   [X.XX µs X.XX µs X.XX µs]")
    echo "$output" | grep -E "time:\s+\[" | while read -r line; do
        # Extract benchmark name and timing
        local test_name=$(echo "$line" | awk '{print $1}')
        local time_range=$(echo "$line" | grep -oE '\[.*\]')

        # Extract median time and unit
        local median=$(echo "$time_range" | awk '{print $2, $3}')

        echo -e "  ${GREEN}✓${NC} $test_name: ${BOLD}$median${NC}"
    done

    # Extract performance changes if comparing
    if [[ "$COMPARE_MODE" == true ]]; then
        echo ""
        echo "$output" | grep -E "(improved|regressed|no change)" | while read -r line; do
            if echo "$line" | grep -q "improved"; then
                echo -e "    ${GREEN}↑ $line${NC}"
            elif echo "$line" | grep -q "regressed"; then
                echo -e "    ${RED}↓ $line${NC}"
            else
                echo -e "    ${YELLOW}→ $line${NC}"
            fi
        done
    fi
}

# Run a single benchmark
run_benchmark() {
    local bench_name="$1"

    log_benchmark "Running $bench_name..."

    if [[ "$VERBOSE" == true ]]; then
        cargo bench --bench "$bench_name" $BENCHMARK_FLAGS
    else
        local output
        output=$(cargo bench --bench "$bench_name" $BENCHMARK_FLAGS 2>&1)
        extract_metrics "$bench_name" "$output"
    fi
}

# Print PRD performance targets
print_targets() {
    echo ""
    echo -e "${BOLD}${CYAN}========================================${NC}"
    echo -e "${BOLD}${CYAN}PicoFlow Performance Benchmarks${NC}"
    echo -e "${BOLD}${CYAN}========================================${NC}"
    echo ""
    echo -e "${BOLD}PRD Performance Targets (from Section 6.1):${NC}"
    echo -e "  • Binary size: <10MB (stripped)"
    echo -e "  • Runtime memory (idle): <20MB"
    echo -e "  • Runtime memory (10 parallel tasks): <50MB"
    echo -e "  • Task startup latency: ${YELLOW}<100ms${NC}"
    echo -e "  • DAG parsing (100 tasks): ${YELLOW}<50ms${NC}"
    echo -e "  • Support: 1000+ tasks per DAG"
    echo ""

    if [[ "$QUICK_MODE" == true ]]; then
        log_info "Running in QUICK mode (fewer samples, faster execution)"
    fi

    if [[ "$BASELINE_MODE" == true ]]; then
        log_info "Saving results as baseline for future comparisons"
    fi

    if [[ "$COMPARE_MODE" == true ]]; then
        log_info "Comparing against saved baseline"
    fi

    echo ""
}

# Print summary
print_summary() {
    echo ""
    echo -e "${BOLD}${GREEN}========================================${NC}"
    echo -e "${BOLD}${GREEN}Benchmark Summary${NC}"
    echo -e "${BOLD}${GREEN}========================================${NC}"
    echo ""

    if [[ "$BASELINE_MODE" == true ]]; then
        log_success "Baseline saved successfully"
        echo "  Future runs with --compare will show performance changes"
    elif [[ "$COMPARE_MODE" == true ]]; then
        echo "  See performance changes above (improved/regressed/no change)"
    fi

    echo ""
    log_info "Benchmark reports saved to: target/criterion/"
    log_info "View HTML reports: open target/criterion/report/index.html"
    echo ""

    log_success "All benchmarks completed!"
}

# Main execution
main() {
    parse_args "$@"

    print_targets

    # Check if cargo bench is available
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        exit 1
    fi

    # Run all benchmarks
    for bench in "${BENCHMARKS[@]}"; do
        if ! run_benchmark "$bench"; then
            log_error "Benchmark $bench failed"
            exit 1
        fi
        echo ""
    done

    print_summary
}

# Run main function
main "$@"
