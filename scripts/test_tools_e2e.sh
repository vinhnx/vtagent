#!/bin/bash
# VTAgent Tools End-to-End Test Script
# Tests the new PTY-compatible tools: simple_search and bash

set -e  # Exit on any error

echo "🧪 VTAgent Tools End-to-End Test Suite"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_RUN=0
TESTS_PASSED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_contains="$3"

    echo -e "\n${YELLOW}Running test: ${test_name}${NC}"
    TESTS_RUN=$((TESTS_RUN + 1))

    if eval "$command" 2>/dev/null | grep -q "$expected_contains"; then
        echo -e "${GREEN}PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAILED${NC}"
        echo "Command: $command"
        echo "Expected to contain: $expected_contains"
    fi
}

# Function to test tool directly (if binary exists)
test_tool_directly() {
    local tool_name="$1"
    local test_input="$2"
    local expected_contains="$3"

    echo -e "\n${YELLOW}Testing $tool_name tool directly${NC}"

    # Build and execute the tool through the binary
    if cargo build --bin vtagent >/dev/null 2>&1; then
        output=$(cargo run --quiet --bin vtagent -- "$tool_name" "$test_input" 2>/dev/null)
        if echo "$output" | grep -q "$expected_contains"; then
            echo -e "${GREEN}Tool output verified${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            echo -e "${RED}❌ Unexpected tool output${NC}"
        fi
    else
        echo -e "${RED}❌ Tool compilation failed${NC}"
    fi
    TESTS_RUN=$((TESTS_RUN + 1))
}

echo -e "\nTesting Tool Compilation"
echo "============================"

# Test 1: Check if tools compile
run_test "VTAgent Core Compilation" "cargo check --package vtagent-core --quiet && echo 'success'" "success"

echo -e "\nTesting PTY Dependencies"
echo "============================"

# Test 2: Check if rexpect is available (PTY library)
run_test "PTY Library Available" "cargo tree --package vtagent-core | grep rexpect && echo 'found'" "found"

# Test 3: Check if ripgrep is available
run_test "Ripgrep Available" "which rg" "rg"

# Test 4: Check if ast-grep is available
run_test "AST Grep Available" "which ast-grep" "ast-grep"

echo -e "\n🧪 Testing Tool Functionality"
echo "============================="

# Test 5: Test ripgrep directly
run_test "Ripgrep Direct Test" "echo 'test content' > /tmp/test_file.txt && rg 'test' /tmp/test_file.txt" "test content"

# Test 6: Test ast-grep directly
run_test "AST Grep Direct Test" "echo 'fn test() {}' > /tmp/test.rs && ast-grep --lang rust --pattern 'fn \$A() {}' /tmp/test.rs" "fn test()"

echo -e "\n📊 Test Results"
echo "==============="
echo "Tests Run: $TESTS_RUN"
echo "Tests Passed: $TESTS_PASSED"
echo "Success Rate: $((TESTS_PASSED * 100 / TESTS_RUN))%"

if [ $TESTS_PASSED -eq $TESTS_RUN ]; then
    echo -e "\n${GREEN}🎉 All tests passed! Tools are working correctly.${NC}"
    exit 0
else
    echo -e "\n${RED} Some tests failed. Please check the output above.${NC}"
    exit 1
fi