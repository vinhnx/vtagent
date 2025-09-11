#!/bin/bash

# Test LMStudio connection
echo "Testing LMStudio connection..."

# Check if LMStudio is running
if curl -s http://localhost:1234/v1/models >/dev/null; then
    echo "✅ LMStudio is running"
    
    # Get model info
    echo "Getting model info:"
    response=$(curl -s http://localhost:1234/v1/models)
    echo "$response" | jq .
    
    # Check if any models are loaded
    model_count=$(echo "$response" | jq '.data | length')
    if [ "$model_count" -gt 0 ]; then
        echo "Found $model_count models"
        # Get the first model
        first_model=$(echo "$response" | jq -r '.data[0].id')
        echo "Using model: $first_model"
        
        # Test chat completion (more appropriate for chat models)
        echo -e "\nTesting chat completion:"
        curl -s http://localhost:1234/v1/chat/completions \
            -H "Content-Type: application/json" \
            -d "{
                \"model\": \"$first_model\",
                \"messages\": [
                    {\"role\": \"user\", \"content\": \"Say hello world in one word\"}
                ],
                \"max_tokens\": 50,
                \"temperature\": 0.7
            }" | jq .
    else
        echo "❌ No models are loaded in LMStudio"
        echo "Please load a model in LMStudio before testing"
    fi
else
    echo "❌ LMStudio is not running or not accessible at http://localhost:1234"
    echo "Please make sure LMStudio is running with a model loaded"
fi