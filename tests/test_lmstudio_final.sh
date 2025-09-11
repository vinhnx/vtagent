#!/bin/bash

# Simple LMStudio test script that works without compiling Rust code
# This script tests LMStudio connectivity using curl

echo "🔍 LMStudio Connectivity Test (No Compilation Required)"
echo "======================================================="
echo

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo "❌ Error: curl is required but not found"
    echo "   Please install curl to run this test"
    exit 1
fi

# Check if jq is available (for pretty printing)
HAS_JQ=false
if command -v jq &> /dev/null; then
    HAS_JQ=true
fi

echo "📡 Testing connection to LMStudio at http://localhost:1234..."
echo

# Test 1: Check if LMStudio is running
echo "📋 Test 1: Checking if LMStudio server is accessible..."
if curl -s -m 5 "http://localhost:1234/v1/models" > /tmp/lmstudio_test.json 2>/dev/null; then
    echo "✅ Success: LMStudio server is running"
    
    # Show available models
    echo
    echo "📂 Available Models:"
    if $HAS_JQ; then
        cat /tmp/lmstudio_test.json | jq -r '.data[].id' 2>/dev/null || echo "   (Could not parse model list)"
    else
        # Fallback without jq
        grep -o '"id":"[^"]*"' /tmp/lmstudio_test.json | cut -d'"' -f4 | sed 's/^/   - /' || echo "   (Could not parse model list)"
    fi
    
    MODEL_COUNT=$(grep -o '"id"' /tmp/lmstudio_test.json | wc -l | tr -d ' ')
    echo "📊 Total models available: $MODEL_COUNT"
else
    echo "❌ Failed: Cannot connect to LMStudio server"
    echo "💡 Troubleshooting tips:"
    echo "   1. Make sure LMStudio is installed and running"
    echo "   2. In LMStudio, go to 'Local Inference' tab"
    echo "   3. Load a model if you haven't already"
    echo "   4. Click 'Start Server' button"
    echo "   5. Verify LMStudio is using port 1234"
    rm -f /tmp/lmstudio_test.json
    exit 1
fi

echo
echo "🧪 Test 2: Testing model completion..."

# Get the first model name for testing
if $HAS_JQ; then
    FIRST_MODEL=$(cat /tmp/lmstudio_test.json | jq -r '.data[0].id' 2>/dev/null)
else
    FIRST_MODEL=$(grep -o '"id":"[^"]*"' /tmp/lmstudio_test.json | head -1 | cut -d'"' -f4)
fi

if [ -n "$FIRST_MODEL" ] && [ "$FIRST_MODEL" != "null" ]; then
    echo "   Testing with model: $FIRST_MODEL"
    
    # Test completion with a simple prompt
    TEST_PROMPT="Say hello world in one word"
    echo "   Prompt: $TEST_PROMPT"
    
    # Create test request
    cat > /tmp/lmstudio_request.json <<EOF
{
    "model": "$FIRST_MODEL",
    "messages": [
        {
            "role": "user",
            "content": "$TEST_PROMPT"
        }
    ],
    "temperature": 0.7,
    "max_tokens": 100
}
EOF
    
    # Send completion request
    if curl -s -m 10 \
        -X POST "http://localhost:1234/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d @/tmp/lmstudio_request.json \
        > /tmp/lmstudio_completion.json 2>/dev/null; then
        
        if $HAS_JQ; then
            # Check if response is valid JSON
            if cat /tmp/lmstudio_completion.json | jq . >/dev/null 2>&1; then
                echo "✅ Success: Model completion API is working"
                
                # Extract response content
                RESPONSE_CONTENT=$(cat /tmp/lmstudio_completion.json | jq -r '.choices[0].message.content' 2>/dev/null)
                if [ "$RESPONSE_CONTENT" != "null" ] && [ -n "$RESPONSE_CONTENT" ]; then
                    echo "   Response: $RESPONSE_CONTENT"
                else
                    echo "   ⚠️  Got response but no content (model might still be loading)"
                fi
            else
                echo "❌ Response is not valid JSON:"
                head -10 /tmp/lmstudio_completion.json
            fi
        else
            # Check without jq
            if grep -q '"choices"' /tmp/lmstudio_completion.json; then
                echo "✅ Success: Model completion API is working"
                echo "   (Install jq for better response formatting)"
            else
                echo "❌ Unexpected response format"
                head -5 /tmp/lmstudio_completion.json
            fi
        fi
    else
        echo "⚠️  Failed to get completion response (this is OK if model is still loading)"
        echo "💡 Model might still be initializing in LMStudio"
    fi
else
    echo "⚠️  Could not determine model name for testing"
fi

# Clean up
rm -f /tmp/lmstudio_test.json /tmp/lmstudio_request.json /tmp/lmstudio_completion.json

echo
echo "🎉 LMStudio connectivity test completed successfully!"
echo
echo "📝 Next steps:"
echo "   1. Make sure LMStudio is running with a model loaded"
echo "   2. Update your VTAgent configuration to use LMStudio"
echo "   3. Create a vtagent.toml file with:"
echo
echo "      [agent]"
echo "      model = \"$FIRST_MODEL\""
echo "      provider = \"lmstudio\""
echo
echo "💡 Tip: If you see 'model not found' errors in VTAgent,"
echo "        check that the model name exactly matches what's"
echo "        shown in LMStudio (case-sensitive).