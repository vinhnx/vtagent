# Using LMStudio with VTAgent - Quick Start Guide

## Prerequisites

1. **LMStudio**: Download from [https://lmstudio.ai/](https://lmstudio.ai/)
2. **VTAgent**: This repository
3. **Rust**: Install from [https://rustup.rs/](https://rustup.rs/)

## Step 1: Set Up LMStudio

1. **Download and Install LMStudio**
   - Go to [https://lmstudio.ai/](https://lmstudio.ai/)
   - Download the appropriate version for your operating system
   - Install LMStudio

2. **Load a Model**
   - Launch LMStudio
   - Go to the "Local Inference" tab
   - Browse and download a model (recommended: Qwen3 series, Llama 3.1 series, or Mistral series)
   - Popular choices:
     - `Qwen3-4B` - Good balance of performance and resource usage
     - `Qwen3-1.7B` - Lightweight, faster
     - `Llama-3.1-8B` - Well-balanced model
     - `Mistral-7B` - Efficient and capable

3. **Start the Server**
   - After loading a model, click "Start Server"
   - The server will start on `http://localhost:1234`
   - Note the model name that appears in the UI

## Step 2: Configure VTAgent

1. **Create Configuration File**
   - In your project root, create a file named `vtagent.toml`
   - Use this minimal configuration:

```toml
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
```

2. **Customize the Model Name**
   - Replace `qwen3-4b-2507` with the actual model name from LMStudio
   - The model name should match exactly what's shown in LMStudio

## Step 3: Run VTAgent

1. **Start VTAgent**
   ```bash
   cargo run
   ```

2. **Interact with the Agent**
   - Type your queries in the prompt
   - The agent will use your local LMStudio model to generate responses
   - You can ask coding questions, request explanations, or get help with tasks

## Troubleshooting

### Common Issues

1. **Cannot Connect to LMStudio**
   - Make sure LMStudio is running and the server is started
   - Check that LMStudio is listening on `http://localhost:1234`
   - Verify your firewall settings

2. **Model Not Found**
   - Double-check the model name in your `vtagent.toml` file
   - Ensure the model name exactly matches what's shown in LMStudio

3. **Slow Responses**
   - Larger models take longer to respond
   - Consider using a smaller model for faster responses
   - Ensure your system has sufficient RAM

### Testing Connection

You can test the connection manually with curl:

```bash
# Test if LMStudio is running
curl -X GET "http://localhost:1234/v1/models"

# Test a simple completion (replace model name)
curl -X POST "http://localhost:1234/v1/completions" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "your-model-name",
    "prompt": "Say hello world",
    "max_tokens": 100
  }'
```

## Advanced Configuration

### Model-Specific Settings

For different models, you might want to adjust settings:

**For Qwen models:**
```toml
[agent]
model = "qwen3-4b-2507"
provider = "lmstudio"
```

**For Llama models:**
```toml
[agent]
model = "llama-3.1-8b"
provider = "lmstudio"
```

### Custom Port

If you're running LMStudio on a different port:

```toml
[agent]
model = "your-model-name"
provider = "lmstudio"

# You would need to modify the LMStudio provider in the code
# to use a custom base URL
```

## Tips for Best Experience

1. **Choose the Right Model**
   - For coding tasks: Qwen3 or Llama 3.1 series
   - For general tasks: Smaller, faster models
   - Balance performance with resource usage

2. **Monitor Resource Usage**
   - Large models can consume significant RAM
   - Close other applications when running resource-intensive models

3. **Experiment with Settings**
   - Adjust `max_turns` based on your needs
   - Modify `command_timeout_seconds` for longer-running commands

4. **Use Safe Commands**
   - The default safe commands are generally safe
   - Customize the list based on your specific needs

## Next Steps

Once you have the basic setup working:

1. **Try Different Models**
   - Experiment with various models to find what works best for your use case

2. **Enable Multi-Agent Mode**
   - Set `multi_agent.enabled = true` for more complex tasks

3. **Customize Security Settings**
   - Adjust `human_in_the_loop` and other security settings based on your comfort level

4. **Explore Tools**
   - VTAgent comes with various tools for file operations, code analysis, etc.

This guide should get you started with using LMStudio and VTAgent together. The combination provides a powerful local AI coding assistant without requiring internet access or API keys.