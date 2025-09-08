#!/bin/bash

# Test script for multi-agent functionality
# This tests the multi-agent conversation loop without making actual API calls

echo "Testing multi-agent system..."

# Test 1: Check if multi-agent mode is detected
echo "=== Test 1: Multi-agent mode detection ==="
GEMINI_API_KEY="test" echo "hello" | timeout 3s ./target/debug/vtagent --workspace=. chat 2>&1 | grep -E "(Multi-agent|Using multi-agent)" && echo "✅ Multi-agent mode detected" || echo "❌ Multi-agent mode not detected"

# Test 2: Check if orchestrator is initialized
echo -e "\n=== Test 2: Orchestrator initialization ==="
GEMINI_API_KEY="test" echo "hello" | timeout 3s ./target/debug/vtagent --workspace=. chat 2>&1 | grep -E "(orchestrator|Orchestrator)" && echo "✅ Orchestrator initialized" || echo "❌ Orchestrator not initialized"

echo -e "\nTest completed!"
