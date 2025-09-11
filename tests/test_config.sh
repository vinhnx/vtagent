#!/bin/bash

# Test script for VTAgent configuration with different providers

echo "Testing VTAgent configuration with different providers..."

# Test 1: Default configuration (Gemini)
echo "=== Test 1: Default configuration (Gemini) ==="
cargo run -- config

# Test 2: LMStudio configuration
echo "=== Test 2: LMStudio configuration ==="
# Create a temporary config file for LMStudio
cat > /tmp/vtagent_lmstudio.toml << EOF
[agent]
model = "qwen3-4b-2507"
provider = "lmstudio"
max_turns = 1000

[security]
human_in_the_loop = true
verbose_logging = false
log_commands = true

[multi_agent]
enabled = false
strategy = "auto"

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

# Test the configuration
VTAGENT_CONFIG_PATH=/tmp/vtagent_lmstudio.toml cargo run -- config

# Clean up
rm /tmp/vtagent_lmstudio.toml

# Test 3: OpenAI configuration
echo "=== Test 3: OpenAI configuration ==="
cat > /tmp/vtagent_openai.toml << EOF
[agent]
model = "gpt-5"
provider = "openai"
max_turns = 1000

[security]
human_in_the_loop = true
verbose_logging = false
log_commands = true

[multi_agent]
enabled = false
strategy = "auto"

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

# Test the configuration
VTAGENT_CONFIG_PATH=/tmp/vtagent_openai.toml cargo run -- config

# Clean up
rm /tmp/vtagent_openai.toml

echo "=== All tests completed ==="