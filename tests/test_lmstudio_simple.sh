#!/bin/bash

# Simple test script to verify LMStudio connectivity
# This script assumes LMStudio is running on the default port (1234)

echo "=== LMStudio Connectivity Test ==="

# Check if LMStudio is running
echo "Checking if LMStudio is accessible at http://localhost:1234/v1..."

# Test the models endpoint
curl -s -X GET "http://localhost:1234/v1/models" > /tmp/lmstudio_response.json 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ Successfully connected to LMStudio!"
    
    # Check if we got a valid response
    if grep -q '"data"' /tmp/lmstudio_response.json; then
        echo "✅ Valid response received from LMStudio:"
        cat /tmp/lmstudio_response.json | jq '.data[0]' 2>/dev/null || echo "Models available (see full response in /tmp/lmstudio_response.json)"
    else
        echo "⚠️  Connected but received unexpected response:"
        cat /tmp/lmstudio_response.json
    fi
else
    echo "❌ Failed to connect to LMStudio"
    echo "Make sure LMStudio is running and accessible at http://localhost:1234"
fi

# Clean up
rm -f /tmp/lmstudio_response.json

echo ""
echo "=== LMStudio Setup Instructions ==="
echo "1. Download and install LMStudio from https://lmstudio.ai/"
echo "2. Launch LMStudio"
echo "3. Go to the 'Local Inference' tab"
echo "4. Select or download a model (e.g., Qwen3, Llama3.1, etc.)"
echo "5. Click 'Start Server'"
echo "6. The server should start on http://localhost:1234"
echo ""
echo "=== VTAgent Configuration ==="
echo "Create a 'vtagent.toml' file with:"
echo ""
echo "[agent]"
echo "model = \"your-model-name\""
echo "provider = \"lmstudio\""
echo ""
echo "Replace 'your-model-name' with the actual model name from LMStudio."