#!/bin/bash

# Simplest possible setup for VTAgent with LMStudio
# This script creates the minimal configuration and tests the setup

echo "🚀 Setting up VTAgent with LMStudio (Simplest Configuration)"

# Create the minimal configuration file
echo "📝 Creating minimal configuration file..."

cat > vtagent.toml << 'EOF'
# VTAgent Configuration - Minimal LMStudio Setup

[agent]
model = "qwen3-4b-2507"      # Replace with your actual model name
provider = "lmstudio"
max_turns = 1000

[security]
human_in_the_loop = true

[multi_agent]
enabled = false

[tools]
default_policy = "prompt"

[commands]
safe_commands = ["ls", "pwd", "cat", "grep", "git status", "git diff"]

[pty]
enabled = true
default_rows = 24
default_cols = 80
max_sessions = 10
command_timeout_seconds = 300
EOF

echo "✅ Created vtagent.toml with minimal configuration"

# Test LMStudio connection
echo "🔍 Testing LMStudio connection..."
if command -v curl >/dev/null 2>&1; then
    if curl -s -m 5 "http://localhost:1234/v1/models" >/dev/null 2>&1; then
        echo "✅ LMStudio is accessible!"
        echo "   You're ready to use VTAgent with LMStudio!"
        echo ""
        echo "📋 Next steps:"
        echo "   1. Make sure LMStudio is running with a model loaded"
        echo "   2. Verify the model name in vtagent.toml matches your LMStudio model"
        echo "   3. Run: cargo run"
        echo ""
        echo "💡 Tip: If you get 'model not found' errors, check the model name"
        echo "        in LMStudio and update vtagent.toml accordingly."
    else
        echo "❌ Cannot connect to LMStudio"
        echo "   Please make sure LMStudio is running and the server is started."
        echo "   The server should be accessible at http://localhost:1234"
        echo ""
        echo "Troubleshooting:"
        echo "   1. Launch LMStudio"
        echo "   2. Go to 'Local Inference' tab"
        echo "   3. Load a model if you haven't already"
        echo "   4. Click 'Start Server'"
        echo "   5. Run this script again"
    fi
else
    echo "⚠️  curl is not installed, skipping connection test"
    echo "   Please install curl to enable connection testing"
fi

echo ""
echo "🎉 Setup complete!"
echo "   Configuration file: vtagent.toml"
echo "   To use VTAgent with LMStudio:"
echo "   1. Ensure LMStudio is running with a model loaded"
echo "   2. Run: cargo run"
echo "   3. Start chatting with your local AI assistant!"