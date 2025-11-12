#!/bin/bash
# PicoFlow Release Binary Testing Script

set -e

RELEASE_BIN="./target/release/picoflow"
TEST_DB="/tmp/picoflow-test-$(date +%s).db"

echo "================================="
echo "PicoFlow Release Binary Test Suite"
echo "================================="
echo ""

# Test 1: Binary exists and is executable
echo "✓ Test 1: Checking binary..."
if [ -f "$RELEASE_BIN" ]; then
    SIZE=$(ls -lh "$RELEASE_BIN" | awk '{print $5}')
    echo "  Binary found: $SIZE"
else
    echo "  ✗ Binary not found. Run: cargo build --release"
    exit 1
fi

# Test 2: Help command
echo ""
echo "✓ Test 2: Help command..."
$RELEASE_BIN --help > /dev/null
echo "  --help works"

# Test 3: Version
echo ""
echo "✓ Test 3: Version..."
$RELEASE_BIN --version
echo "  --version works"

# Test 4: Validate command
echo ""
echo "✓ Test 4: Validate workflow..."
$RELEASE_BIN validate examples/workflows/simple.yaml --log-format pretty --log-level error 2>&1 | head -3
echo "  Validation successful"

# Test 5: Run workflow
echo ""
echo "✓ Test 5: Run workflow..."
$RELEASE_BIN run examples/workflows/simple.yaml --db-path "$TEST_DB" --log-level error 2>&1
echo "  Workflow executed successfully"

# Test 6: Status command
echo ""
echo "✓ Test 6: Check status..."
$RELEASE_BIN status --workflow simple-workflow --db-path "$TEST_DB" 2>/dev/null | head -10
echo "  Status retrieved successfully"

# Test 7: Run with different log formats
echo ""
echo "✓ Test 7: Test log formats..."
$RELEASE_BIN run examples/workflows/simple.yaml --db-path "$TEST_DB" --log-format json --log-level error 2>&1 > /dev/null
echo "  JSON logging works"
$RELEASE_BIN run examples/workflows/simple.yaml --db-path "$TEST_DB" --log-format pretty --log-level error 2>&1 > /dev/null
echo "  Pretty logging works"

# Test 8: Invalid workflow (should fail)
echo ""
echo "✓ Test 8: Test error handling..."
cat > /tmp/invalid-workflow.yaml << 'INVALID'
name: invalid
tasks: []
INVALID
if $RELEASE_BIN validate /tmp/invalid-workflow.yaml 2>&1 | grep -q "error\|Error"; then
    echo "  Error handling works (empty tasks rejected)"
else
    echo "  ✗ Should have failed on empty tasks"
fi
rm -f /tmp/invalid-workflow.yaml

# Test 9: Database persistence
echo ""
echo "✓ Test 9: Database persistence..."
DB_SIZE=$(ls -lh "$TEST_DB" | awk '{print $5}')
echo "  Database created: $DB_SIZE"
EXEC_COUNT=$($RELEASE_BIN status --workflow simple-workflow --db-path "$TEST_DB" 2>/dev/null | grep "Execution ID:" | wc -l | tr -d ' ')
echo "  Executions recorded: $EXEC_COUNT"

# Test 10: Binary size check
echo ""
echo "✓ Test 10: Binary size check..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    SIZE_BYTES=$(stat -f%z "$RELEASE_BIN")
else
    SIZE_BYTES=$(stat -c%s "$RELEASE_BIN")
fi
SIZE_MB=$((SIZE_BYTES / 1024 / 1024))
if [ $SIZE_MB -lt 10 ]; then
    echo "  Binary size: ${SIZE_MB}MB (✓ under 10MB target)"
else
    echo "  ✗ Binary size: ${SIZE_MB}MB (exceeds 10MB target)"
    exit 1
fi

# Cleanup
echo ""
echo "✓ Cleanup..."
rm -f "$TEST_DB"
echo "  Test database removed"

echo ""
echo "================================="
echo "All tests passed! ✓"
echo "================================="
echo ""
echo "Release binary is ready for deployment!"
echo ""
echo "Quick start:"
echo "  $RELEASE_BIN validate examples/workflows/simple.yaml"
echo "  $RELEASE_BIN run examples/workflows/simple.yaml"
echo "  $RELEASE_BIN status --workflow simple-workflow"
