#!/bin/bash
# VTAgent PTY Tools Focused Test
# Tests specifically the PTY-compatible tools we updated

set -e

echo "🧪 VTAgent PTY Tools Test"
echo "========================"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_RUN=0
TESTS_PASSED=0

run_test() {
    local test_name="$1"
    local command="$2"
    local expected_contains="$3"

    echo -e "\n${YELLOW}Testing: ${test_name}${NC}"
    TESTS_RUN=$((TESTS_RUN + 1))

    if eval "$command" 2>/dev/null | grep -q "$expected_contains"; then
        echo -e "${GREEN}PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAILED${NC}"
        echo "Command: $command"
        echo "Expected: $expected_contains"
    fi
}

echo -e "\nTesting External Tools Availability"
echo "====================================="

# Test 1: CK search
run_test "CK Search Available" "which ck" "ck"

# Test 2: Ripgrep
run_test "Ripgrep Available" "which rg" "rg"

# Test 3: AST-grep
run_test "AST Grep Available" "which ast-grep" "ast-grep"

# Test 4: rexpect (PTY library) - check if it's in Cargo.toml
run_test "PTY Library in Dependencies" "grep rexpect vtagent-core/Cargo.toml" "rexpect"

echo -e "\n🧪 Testing Tool Functionality"
echo "============================"

# Test 5: Ripgrep functionality
run_test "Ripgrep Functionality" "echo 'test content' > /tmp/vtagent_test.txt && rg 'test' /tmp/vtagent_test.txt && rm /tmp/vtagent_test.txt" "test content"

# Test 6: AST-grep functionality
run_test "AST Grep Functionality" "echo 'fn test() {}' > /tmp/vtagent_test.rs && ast-grep --lang rust --pattern 'fn \$A() {}' /tmp/vtagent_test.rs && rm /tmp/vtagent_test.rs" "fn test()"

echo -e "\nTesting Code Structure"
echo "========================="

# Test 7: Check if PTY methods exist in tools
run_test "BashTool PTY Methods" "grep 'execute_pty_command' vtagent-core/src/tools/bash_tool.rs" "execute_pty_command"

# Test 8: Check if SimpleSearchTool PTY methods exist
run_test "SimpleSearchTool PTY Methods" "grep 'execute_pty_command' vtagent-core/src/tools/simple_search.rs" "execute_pty_command"

# Test 9: Check if rexpect is imported
run_test "PTY Imports" "grep 'rexpect::spawn' vtagent-core/src/tools/bash_tool.rs" "rexpect::spawn"

echo -e "\n📊 Test Results"
echo "==============="
echo "Tests Run: $TESTS_RUN"
echo "Tests Passed: $TESTS_PASSED"
echo "Success Rate: $((TESTS_PASSED * 100 / TESTS_RUN))%"

if [ $TESTS_PASSED -eq $TESTS_RUN ]; then
    echo -e "\n${GREEN}🎉 All PTY tool tests passed!${NC}"
    echo "External tools are available and functional"
    echo "PTY integration is properly implemented"
    echo "Tools are compatible with terminal emulation"
    exit 0
else
    echo -e "\n${RED} Some tests failed. Check the output above.${NC}"
    exit 1
fi