#!/bin/bash

# VTAGENT - DSPy Integration Test Script
# This script tests the DSPy prompt optimizer integration

set -e

echo "VTAGENT - DSPy Integration Test"
echo "==============================="

# Check if API key is set
if [[ -z "$GEMINI_API_KEY" && -z "$GOOGLE_API_KEY" ]]; then
    echo "Error: API key not found!"
    echo ""
    echo "Please set one of these environment variables:"
    echo "  export GEMINI_API_KEY='your_gemini_api_key_here'"
    echo "  export GOOGLE_API_KEY='your_google_api_key_here'"
    echo ""
    echo "Get your API key from: https://aistudio.google.com/app/apikey"
    exit 1
fi

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: Please run this script from the vtagent project root directory"
    exit 1
fi

echo "Testing DSPy prompt optimizer integration..."
echo ""

# Run a simple test to see the DSPy optimizer in action
echo "Running DSPy test with debug output..."
echo "====================================="
echo ""

# We'll use cargo run with debug logging to see the optimizer output
# The test will show how a simple prompt gets transformed by DSPy
RUST_LOG=debug cargo run --bin dspy-test-runner

echo ""
echo "DSPy Integration Test Complete!"
echo "==============================="
echo ""
echo "If you saw [OPTIMIZER] output with a structured prompt,"
echo "then the DSPy integration is working correctly."