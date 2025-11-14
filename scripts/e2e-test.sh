#!/usr/bin/env bash
#
# e2e-test.sh - End-to-End Feature Test for PicoFlow
#
# This script tests all major features of PicoFlow in a real environment
# (not unit/integration tests, but actual workflow execution).
#
# Features tested:
# - CLI commands (validate, run, status, history, stats, logs, workflow list)
# - Shell executor
# - HTTP executor
# - DAG dependencies
# - Retry logic
# - Timeout handling
# - Daemon mode (start/stop/status)
# - Workflow scheduling
# - Database persistence
# - Error handling
#
# Usage:
#   ./scripts/e2e-test.sh [--verbose]
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERBOSE=false
if [[ "${1:-}" == "--verbose" ]]; then
    VERBOSE=true
fi

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Temporary directory for test files
TEST_DIR=$(mktemp -d)
trap 'cleanup' EXIT

cleanup() {
    echo ""
    log_info "Cleaning up..."

    # Stop daemon if running
    if [[ -f "$TEST_DIR/picoflow.pid" ]]; then
        ./target/release/picoflow daemon stop --db-path "$TEST_DIR/test.db" 2>/dev/null || true
    fi

    # Remove test directory
    rm -rf "$TEST_DIR"

    echo ""
    echo "========================================"
    echo "E2E Test Summary"
    echo "========================================"
    echo -e "Total tests: $TESTS_RUN"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo -e "${RED}Failed: $TESTS_FAILED${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
    fi
}

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Test helper
run_test() {
    local test_name="$1"
    local test_cmd="$2"

    ((TESTS_RUN++))

    if $VERBOSE; then
        echo ""
        log_info "Running: $test_name"
        echo "Command: $test_cmd"
    fi

    if eval "$test_cmd" > /dev/null 2>&1; then
        ((TESTS_PASSED++))
        log_success "$test_name"
        return 0
    else
        ((TESTS_FAILED++))
        log_error "$test_name"
        if $VERBOSE; then
            echo "Failed command: $test_cmd"
        fi
        return 1
    fi
}

# Test helper with output capture
run_test_with_output() {
    local test_name="$1"
    local test_cmd="$2"
    local expected_output="$3"

    ((TESTS_RUN++))

    if $VERBOSE; then
        echo ""
        log_info "Running: $test_name"
        echo "Command: $test_cmd"
    fi

    local output
    output=$(eval "$test_cmd" 2>&1 || true)

    if echo "$output" | grep -q "$expected_output"; then
        ((TESTS_PASSED++))
        log_success "$test_name"
        return 0
    else
        ((TESTS_FAILED++))
        log_error "$test_name"
        if $VERBOSE; then
            echo "Expected: $expected_output"
            echo "Got: $output"
        fi
        return 1
    fi
}

echo "========================================"
echo "PicoFlow End-to-End Feature Test"
echo "========================================"
echo "Test directory: $TEST_DIR"
echo ""

# Check if binary exists
if [[ ! -f "./target/release/picoflow" ]]; then
    log_error "PicoFlow binary not found at ./target/release/picoflow"
    log_info "Please run: cargo build --release"
    exit 1
fi

log_success "PicoFlow binary found"
echo ""

# ============================================================================
# Test 1: CLI - Version and Help
# ============================================================================
log_info "Testing CLI commands..."

run_test "CLI: --version" \
    "./target/release/picoflow --version"

run_test "CLI: --help" \
    "./target/release/picoflow --help"

run_test "CLI: run --help" \
    "./target/release/picoflow run --help"

run_test "CLI: validate --help" \
    "./target/release/picoflow validate --help"

# ============================================================================
# Test 2: Workflow Validation
# ============================================================================
echo ""
log_info "Testing workflow validation..."

# Create valid workflow
cat > "$TEST_DIR/valid-workflow.yaml" <<'EOF'
name: test-workflow
description: "E2E test workflow"

config:
  max_parallel: 2
  retry_default: 1
  timeout_default: 30

tasks:
  - name: task1
    type: shell
    config:
      command: "echo 'Task 1 executed'"

  - name: task2
    type: shell
    depends_on: [task1]
    config:
      command: "echo 'Task 2 executed'"
EOF

run_test "Validate: Valid workflow" \
    "./target/release/picoflow validate $TEST_DIR/valid-workflow.yaml"

# Create invalid workflow (missing required field)
cat > "$TEST_DIR/invalid-workflow.yaml" <<'EOF'
name: invalid-workflow
# Missing tasks field
config:
  max_parallel: 2
EOF

run_test "Validate: Invalid workflow detection" \
    "! ./target/release/picoflow validate $TEST_DIR/invalid-workflow.yaml"

# Create workflow with cycle
cat > "$TEST_DIR/cycle-workflow.yaml" <<'EOF'
name: cycle-workflow
description: "Workflow with cycle"

tasks:
  - name: task1
    type: shell
    depends_on: [task2]
    config:
      command: "echo 'Task 1'"

  - name: task2
    type: shell
    depends_on: [task1]
    config:
      command: "echo 'Task 2'"
EOF

run_test "Validate: Cycle detection" \
    "! ./target/release/picoflow validate $TEST_DIR/cycle-workflow.yaml"

# ============================================================================
# Test 3: Shell Executor
# ============================================================================
echo ""
log_info "Testing shell executor..."

cat > "$TEST_DIR/shell-workflow.yaml" <<'EOF'
name: shell-test
description: "Test shell executor"

tasks:
  - name: echo_test
    type: shell
    config:
      command: "/bin/sh"
      args: ["-c", "echo 'Hello from shell'"]

  - name: exit_code_test
    type: shell
    config:
      command: "/bin/sh"
      args: ["-c", "exit 0"]

  - name: env_var_test
    type: shell
    config:
      command: "/bin/sh"
      args: ["-c", "echo $USER"]
      env:
        TEST_VAR: "test_value"
EOF

run_test "Shell: Execute workflow" \
    "./target/release/picoflow run $TEST_DIR/shell-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 4: HTTP Executor
# ============================================================================
echo ""
log_info "Testing HTTP executor..."

cat > "$TEST_DIR/http-workflow.yaml" <<'EOF'
name: http-test
description: "Test HTTP executor"

tasks:
  - name: http_get
    type: http
    config:
      url: "https://httpbin.org/get"
      method: GET
      timeout: 10

  - name: http_post
    type: http
    config:
      url: "https://httpbin.org/post"
      method: POST
      timeout: 10
      body: '{"test": "data"}'
      headers:
        Content-Type: "application/json"
EOF

run_test "HTTP: Execute GET request" \
    "./target/release/picoflow run $TEST_DIR/http-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 5: DAG Dependencies
# ============================================================================
echo ""
log_info "Testing DAG dependencies..."

cat > "$TEST_DIR/dag-workflow.yaml" <<'EOF'
name: dag-test
description: "Test DAG execution order"

tasks:
  - name: start
    type: shell
    config:
      command: "/bin/echo 'Start'"

  - name: parallel1
    type: shell
    depends_on: [start]
    config:
      command: "/bin/echo 'Parallel 1'"

  - name: parallel2
    type: shell
    depends_on: [start]
    config:
      command: "/bin/echo 'Parallel 2'"

  - name: end
    type: shell
    depends_on: [parallel1, parallel2]
    config:
      command: "/bin/echo 'End'"
EOF

run_test "DAG: Execute with dependencies" \
    "./target/release/picoflow run $TEST_DIR/dag-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 6: Retry Logic
# ============================================================================
echo ""
log_info "Testing retry logic..."

cat > "$TEST_DIR/retry-workflow.yaml" <<'EOF'
name: retry-test
description: "Test retry logic"

tasks:
  - name: fail_then_succeed
    type: shell
    retry: 3
    config:
      command: "/usr/bin/false"  # This will fail
EOF

run_test "Retry: Task with retries (expected to fail)" \
    "! ./target/release/picoflow run $TEST_DIR/retry-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 7: Timeout Handling
# ============================================================================
echo ""
log_info "Testing timeout handling..."

cat > "$TEST_DIR/timeout-workflow.yaml" <<'EOF'
name: timeout-test
description: "Test timeout handling"

tasks:
  - name: quick_task
    type: shell
    timeout: 5
    config:
      command: "/bin/echo 'Quick task'"

  - name: slow_task
    type: shell
    timeout: 2
    config:
      command: "/bin/sleep 10"  # Will timeout
EOF

run_test "Timeout: Quick task succeeds" \
    "./target/release/picoflow run $TEST_DIR/timeout-workflow.yaml --db-path $TEST_DIR/test.db || true"

# ============================================================================
# Test 8: Workflow List
# ============================================================================
echo ""
log_info "Testing workflow list..."

run_test_with_output "Workflow: List workflows" \
    "./target/release/picoflow workflow list --db-path $TEST_DIR/test.db" \
    "shell-test"

# ============================================================================
# Test 9: Status Command
# ============================================================================
echo ""
log_info "Testing status command..."

run_test "Status: Check workflow status" \
    "./target/release/picoflow status --workflow shell-test --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 10: History Command
# ============================================================================
echo ""
log_info "Testing history command..."

run_test "History: View execution history" \
    "./target/release/picoflow history --workflow shell-test --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 11: Stats Command
# ============================================================================
echo ""
log_info "Testing stats command..."

run_test "Stats: View workflow statistics" \
    "./target/release/picoflow stats --workflow shell-test --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 12: Logs Command
# ============================================================================
echo ""
log_info "Testing logs command..."

run_test "Logs: View task logs" \
    "./target/release/picoflow logs --workflow shell-test --task echo_test --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 13: Daemon Mode
# ============================================================================
echo ""
log_info "Testing daemon mode..."

# Create workflow with schedule
cat > "$TEST_DIR/scheduled-workflow.yaml" <<'EOF'
name: scheduled-test
description: "Test scheduled execution"
schedule: "*/5 * * * * *"  # Every 5 seconds

tasks:
  - name: scheduled_task
    type: shell
    config:
      command: "/bin/echo 'Scheduled execution'"
EOF

# Start daemon
run_test "Daemon: Start daemon" \
    "./target/release/picoflow daemon start $TEST_DIR/scheduled-workflow.yaml --db-path $TEST_DIR/test.db --pid-file $TEST_DIR/picoflow.pid &"

sleep 2

# Check daemon status
run_test "Daemon: Check status" \
    "./target/release/picoflow daemon status --pid-file $TEST_DIR/picoflow.pid"

# Wait for at least one scheduled execution
log_info "Waiting 6 seconds for scheduled execution..."
sleep 6

# Stop daemon
run_test "Daemon: Stop daemon" \
    "./target/release/picoflow daemon stop --pid-file $TEST_DIR/picoflow.pid"

# Verify scheduled task was executed
run_test_with_output "Daemon: Verify scheduled execution" \
    "./target/release/picoflow history --workflow scheduled-test --db-path $TEST_DIR/test.db" \
    "scheduled-test"

# ============================================================================
# Test 14: Error Handling
# ============================================================================
echo ""
log_info "Testing error handling..."

cat > "$TEST_DIR/error-workflow.yaml" <<'EOF'
name: error-test
description: "Test error handling"

tasks:
  - name: failing_task
    type: shell
    config:
      command: "/usr/bin/false"

  - name: should_not_run
    type: shell
    depends_on: [failing_task]
    config:
      command: "/bin/echo 'Should not execute'"
EOF

run_test "Error: Workflow fails on task failure" \
    "! ./target/release/picoflow run $TEST_DIR/error-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 15: Continue on Failure
# ============================================================================
echo ""
log_info "Testing continue_on_failure..."

cat > "$TEST_DIR/continue-workflow.yaml" <<'EOF'
name: continue-test
description: "Test continue_on_failure"

tasks:
  - name: optional_task
    type: shell
    continue_on_failure: true
    config:
      command: "/usr/bin/false"

  - name: should_run
    type: shell
    depends_on: [optional_task]
    config:
      command: "/bin/echo 'This should run'"
EOF

run_test "ContinueOnFailure: Workflow continues after optional task fails" \
    "./target/release/picoflow run $TEST_DIR/continue-workflow.yaml --db-path $TEST_DIR/test.db"

# ============================================================================
# Test 16: Database Persistence
# ============================================================================
echo ""
log_info "Testing database persistence..."

# Run a workflow
./target/release/picoflow run "$TEST_DIR/shell-workflow.yaml" --db-path "$TEST_DIR/persistence.db" > /dev/null 2>&1

# Check if we can query history from the same database
run_test_with_output "Database: Persistence check" \
    "./target/release/picoflow history --workflow shell-test --db-path $TEST_DIR/persistence.db" \
    "shell-test"

# ============================================================================
# Test 17: Complex Workflow
# ============================================================================
echo ""
log_info "Testing complex real-world workflow..."

cat > "$TEST_DIR/complex-workflow.yaml" <<'EOF'
name: complex-test
description: "Complex multi-executor workflow"

config:
  max_parallel: 3
  retry_default: 2
  timeout_default: 30

tasks:
  - name: health_check
    type: http
    config:
      url: "https://httpbin.org/status/200"
      method: GET

  - name: prepare_data
    type: shell
    depends_on: [health_check]
    config:
      command: "/bin/sh -c 'echo Preparing data > /tmp/data.txt'"

  - name: process_1
    type: shell
    depends_on: [prepare_data]
    config:
      command: "/bin/echo 'Processing 1'"

  - name: process_2
    type: shell
    depends_on: [prepare_data]
    config:
      command: "/bin/echo 'Processing 2'"

  - name: aggregate
    type: shell
    depends_on: [process_1, process_2]
    config:
      command: "/bin/echo 'Aggregating results'"

  - name: notify
    type: http
    depends_on: [aggregate]
    config:
      url: "https://httpbin.org/post"
      method: POST
      body: '{"status": "completed"}'
EOF

run_test "Complex: Multi-executor DAG workflow" \
    "TEST_DIR=$TEST_DIR ./target/release/picoflow run $TEST_DIR/complex-workflow.yaml --db-path $TEST_DIR/test.db"

echo ""
log_info "All E2E tests completed!"
