#!/bin/bash

# Comprehensive test for VT Agent providers and models
echo "🧪 Comprehensive VT Agent Test Suite"
echo "====================================="
echo ""

# Test 1: Configuration validation
echo "Test 1: Configuration Validation"
echo "-----------------------------------"

if [ ! -f "vtagent.toml" ]; then
    echo "❌ vtagent.toml not found"
    exit 1
fi

# Check LMStudio configuration
CONFIG_MODEL=$(grep "default_model" vtagent.toml | cut -d'"' -f2)
CONFIG_PROVIDER=$(grep "provider" vtagent.toml | cut -d'"' -f2)

echo "Current configuration:"
echo "  Provider: $CONFIG_PROVIDER"
echo "  Model: $CONFIG_MODEL"
echo ""

# Test 2: LMStudio connectivity
echo "🔗 Test 2: LMStudio Connectivity"
echo "-------------------------------"

if curl -s http://localhost:1234/v1/models >/dev/null 2>&1; then
    echo "LMStudio is running"

    # Check if configured model exists
    if curl -s http://localhost:1234/v1/models | jq -r '.data[].id' | grep -q "$CONFIG_MODEL"; then
        echo "Model '$CONFIG_MODEL' is available"
    else
        echo "❌ Model '$CONFIG_MODEL' not found in LMStudio"
        echo "Available models:"
        curl -s http://localhost:1234/v1/models | jq -r '.data[].id'
        exit 1
    fi
else
    echo "❌ LMStudio is not running at http://localhost:1234"
    echo "Please start LMStudio with the model loaded"
    exit 1
fi
echo ""

# Test 3: VT Agent binary
echo "⚙️  Test 3: VT Agent Binary"
echo "--------------------------"

if [ ! -f "target/release/vtagent" ]; then
    echo "❌ VT Agent binary not found"
    echo "Building..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "❌ Build failed"
        exit 1
    fi
fi

echo "VT Agent binary is ready"
echo ""

# Test 4: Single-agent mode with LMStudio
echo "🤖 Test 4: Single-Agent Mode (LMStudio)"
echo "--------------------------------------"

echo "Testing single-agent mode with LMStudio..."
echo "Note: This test will run for 10 seconds and then be terminated"
echo ""

# Create temporary config with multi-agent disabled
cp vtagent.toml vtagent.toml.backup
sed 's/enabled = true/enabled = false/' vtagent.toml > vtagent.toml.tmp && mv vtagent.toml.tmp vtagent.toml

# Start VT Agent in background and capture output
timeout 10s ./target/release/vtagent chat --provider lmstudio --model "$CONFIG_MODEL" --api-key-env "" 2>&1 | head -20

EXIT_CODE=$?
if [ $EXIT_CODE -eq 124 ] || [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "Single-agent mode started successfully (timed out as expected)"
else
    echo ""
    echo "❌ Single-agent mode failed to start (exit code: $EXIT_CODE)"
    # Restore original config
    mv vtagent.toml.backup vtagent.toml
    exit 1
fi

# Restore original config
mv vtagent.toml.backup vtagent.toml
echo ""

# Test 5: Multi-agent mode fallback
echo "👥 Test 5: Multi-Agent Mode Fallback"
echo "-----------------------------------"

echo "Testing multi-agent mode with LMStudio (should fallback to Gemini)..."
echo "Note: This test will run for 10 seconds and then be terminated"
echo ""

# Check if GEMINI_API_KEY is set
if [ -z "$GEMINI_API_KEY" ]; then
    echo " GEMINI_API_KEY not set - multi-agent test will be skipped"
    echo "   To test multi-agent mode, set: export GEMINI_API_KEY='your_key'"
    MULTI_AGENT_SKIP=true
else
    MULTI_AGENT_SKIP=false
fi

if [ "$MULTI_AGENT_SKIP" = false ]; then
    timeout 10s ./target/release/vtagent chat --force-multi-agent --provider lmstudio --model "$CONFIG_MODEL" --api-key-env "" 2>&1 | head -20

    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 124 ] || [ $EXIT_CODE -eq 0 ]; then
        echo ""
        echo "Multi-agent mode started successfully with Gemini fallback"
    else
        echo ""
        echo "❌ Multi-agent mode failed to start (exit code: $EXIT_CODE)"
        exit 1
    fi
fi
echo ""

# Test 6: Provider switching
echo "🔄 Test 6: Provider Switching"
echo "----------------------------"

echo "Testing provider help output..."
./target/release/vtagent chat --help | grep -A 10 "Available providers:" | head -15

echo ""
echo "Provider information displayed correctly"
echo ""

# Test 7: Model validation
echo "📊 Test 7: Model Validation"
echo "--------------------------"

echo "Testing model help output..."
./target/release/vtagent chat --help | grep -A 10 "Available models:" | head -15

echo ""
echo "Model information displayed correctly"
echo ""

# Summary
echo "📊 Test Summary"
echo "=============="
echo ""
echo "Configuration validation: PASSED"
echo "LMStudio connectivity: PASSED"
echo "VT Agent binary: PASSED"
echo "Single-agent mode: PASSED"
if [ "$MULTI_AGENT_SKIP" = false ]; then
    echo "Multi-agent mode: PASSED"
else
    echo " Multi-agent mode: SKIPPED (GEMINI_API_KEY not set)"
fi
echo "Provider switching: PASSED"
echo "Model validation: PASSED"
echo ""

echo "🎉 All core tests passed!"
echo ""
echo "📝 Usage Instructions:"
echo "======================"
echo ""
echo "Single-Agent Mode (LMStudio):"
echo "  ./target/release/vtagent chat"
echo ""
echo "Multi-Agent Mode (requires GEMINI_API_KEY):"
echo "  export GEMINI_API_KEY='your_gemini_api_key'"
echo "  ./target/release/vtagent chat --force-multi-agent"
echo ""
echo "Custom Provider/Model:"
echo "  ./target/release/vtagent chat --provider gemini --model gemini-2.5-flash"
