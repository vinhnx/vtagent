#!/bin/bash

# Simple LMStudio test - working version

echo "🔍 Testing LMStudio connection..."

# Test if LMStudio is running
if curl -s -m 5 "http://localhost:1234/v1/models" > /tmp/models.json 2>/dev/null; then
    echo "LMStudio is running!"

    # Show first few models
    echo "Available models:"
    if command -v jq &> /dev/null; then
        cat /tmp/models.json | jq -r '.data[].id' | head -3
    else
        grep -o '"id":"[^"]*"' /tmp/models.json | head -3 | cut -d'"' -f4
    fi

    # Clean up
    rm -f /tmp/models.json
else
    echo "❌ Cannot connect to LMStudio"
    echo "Make sure LMStudio is running and the server is started"
    exit 1
fi

echo ""
echo "📝 To configure VTAgent for LMStudio, create a vtagent.toml file with:"
echo ""
echo "[agent]"
echo "model = \"qwen/qwen3-4b-2507\"  # Use the model name from above"
echo "provider = \"lmstudio\""
echo ""
echo "Then run: cargo run"