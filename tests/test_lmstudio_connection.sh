#!/bin/bash

# Simple LMStudio connection test
# This script tests if LMStudio is accessible and responding

echo "🔍 Testing LMStudio Connection..."

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo "❌ curl is required but not found. Please install curl."
    exit 1
fi

# Test connection to LMStudio
echo "📡 Connecting to http://localhost:1234/v1/models..."
response=$(curl -s -m 5 -X GET "http://localhost:1234/v1/models" 2>&1)

# Check if the request was successful
if [[ $? -eq 0 ]]; then
    echo "✅ Successfully connected to LMStudio!"
    
    # Try to parse the response as JSON
    if echo "$response" | jq . >/dev/null 2>&1; then
        echo "📄 Response is valid JSON:"
        echo "$response" | jq '.data[] | {id, object}' 2>/dev/null || echo "$response" | head -10
        
        # Count available models
        model_count=$(echo "$response" | jq '.data | length' 2>/dev/null)
        if [[ $? -eq 0 && -n "$model_count" ]]; then
            echo "📊 Found $model_count available model(s)"
        fi
    else
        echo "⚠️  Received response but it's not valid JSON:"
        echo "$response" | head -10
        echo "..."
    fi
else
    echo "❌ Failed to connect to LMStudio"
    echo "   Error: $response"
    echo ""
    echo "📋 Troubleshooting tips:"
    echo "   1. Make sure LMStudio is running"
    echo "   2. Check that the server is started in LMStudio (Local Inference tab)"
    echo "   3. Verify LMStudio is using the default port (1234)"
    echo "   4. Check your firewall settings"
    echo ""
    echo "🔗 LMStudio should be accessible at: http://localhost:1234"
    exit 1
fi

echo ""
echo "🎉 LMStudio test completed successfully!"
echo "📝 Next steps:"
echo "   1. Create a vtagent.toml file with your configuration"
echo "   2. Run: cargo run"
echo "   3. Start chatting with your local AI assistant!"