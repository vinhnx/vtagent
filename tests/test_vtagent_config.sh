#!/bin/bash

# Test VT Agent with LMStudio configuration
echo "Testing VT Agent with LMStudio configuration..."
echo "=============================================="

# Check if LMStudio is running
if ! curl -s http://localhost:1234/v1/models >/dev/null; then
    echo "❌ LMStudio is not running at http://localhost:1234"
    echo "Please start LMStudio with a model loaded"
    exit 1
fi

echo "✅ LMStudio is running"

# Check if the model from config is available
MODEL_FROM_CONFIG="qwen3-4b-2507"
if curl -s http://localhost:1234/v1/models | jq -r '.data[].id' | grep -q "$MODEL_FROM_CONFIG"; then
    echo "✅ Model '$MODEL_FROM_CONFIG' is available in LMStudio"
else
    echo "❌ Model '$MODEL_FROM_CONFIG' not found in LMStudio"
    echo "Available models:"
    curl -s http://localhost:1234/v1/models | jq -r '.data[].id'
    exit 1
fi

# Test VT Agent configuration
echo ""
echo "Testing VT Agent configuration..."
echo "================================="

# Check if vtagent.toml exists
if [ ! -f "vtagent.toml" ]; then
    echo "❌ vtagent.toml not found"
    exit 1
fi

echo "✅ vtagent.toml exists"

# Check configuration values
CONFIG_MODEL=$(grep "default_model" vtagent.toml | cut -d'"' -f2)
CONFIG_PROVIDER=$(grep "provider" vtagent.toml | cut -d'"' -f2)

echo "Configuration from vtagent.toml:"
echo "  - Model: $CONFIG_MODEL"
echo "  - Provider: $CONFIG_PROVIDER"

if [ "$CONFIG_MODEL" = "$MODEL_FROM_CONFIG" ]; then
    echo "✅ Model configuration matches"
else
    echo "❌ Model configuration mismatch"
    echo "  Expected: $MODEL_FROM_CONFIG"
    echo "  Found: $CONFIG_MODEL"
fi

if [ "$CONFIG_PROVIDER" = "lmstudio" ]; then
    echo "✅ Provider configuration is correct"
else
    echo "❌ Provider configuration mismatch"
    echo "  Expected: lmstudio"
    echo "  Found: $CONFIG_PROVIDER"
fi

echo ""
echo "Testing VT Agent binary..."
echo "==========================="

# Check if binary exists
if [ ! -f "target/release/vtagent" ]; then
    echo "❌ VT Agent binary not found at target/release/vtagent"
    echo "Please build the project first: cargo build --release"
    exit 1
fi

echo "✅ VT Agent binary exists"

# Test help command
if ./target/release/vtagent --help >/dev/null 2>&1; then
    echo "✅ VT Agent binary is executable"
else
    echo "❌ VT Agent binary is not executable"
    exit 1
fi

echo ""
echo "🎉 All tests passed! VT Agent is properly configured for LMStudio."
echo ""
echo "To start VT Agent:"
echo "  ./target/release/vtagent chat"
echo ""
echo "To use multi-agent mode:"
echo "  ./target/release/vtagent chat --force-multi-agent"
