# VTAgent Provider Configuration Guide

## Overview

VTAgent now supports multiple AI providers for both single-agent and multi-agent modes. This allows you to use different AI models from Google Gemini, OpenAI, or Anthropic based on your preferences and requirements.

## Supported Providers

### Gemini (Google)
- **Provider ID**: `gemini`
- **Default API Key**: `GEMINI_API_KEY` or `GOOGLE_API_KEY`
- **Models**:
  - `gemini-2.5-flash-lite` - Fastest, most cost-effective
  - `gemini-2.5-flash` - Fast, cost-effective, default for agent/planning/orchestrator
  - `gemini-2.5-pro` - Latest, most capable
  - `gemini-2.0-flash` - Previous generation, fast

### OpenAI
- **Provider ID**: `openai`
- **Default API Key**: `OPENAI_API_KEY`
- **Models**:
  - `gpt-5` - High performance model
  - `gpt-5-mini` - Smaller, faster version and fast and economical

### Anthropic
- **Provider ID**: `anthropic`
- **Default API Key**: `ANTHROPIC_API_KEY`
- **Models**:
  - `claude-sonnet-4-20250514` - Most intelligent model
  - `claude-opus-4-1-20250805` - Powerful model for complex tasks

## Configuration

### Single Agent Mode

Configure the provider for single agent mode in your `vtagent.toml`:

```toml
[agent]
# Set the AI provider
provider = "gemini"  # or "openai" or "anthropic"

# Optional: specify exact model (will use provider defaults if omitted)
default_model = "gemini-2.5-flash"

# Set appropriate API key environment variable
api_key_env = "GEMINI_API_KEY"
```

### Multi-Agent Mode

Configure providers for multi-agent mode:

```toml
[multi_agent]
enabled = true

# Provider for all agents (inherits from [agent] if not specified)
provider = "gemini"

# Orchestrator model (will use provider defaults if omitted)
orchestrator_model = "gemini-2.5-flash"

# Subagent model (will use provider defaults if omitted)
subagent_model = "gemini-2.5-flash-lite"

debug_mode = true  # Enable to see provider/model selection
```

## Provider-Specific Defaults

When models are not explicitly specified, VTAgent uses intelligent provider-specific defaults:

### Gemini Defaults
- **Single Agent**: `gemini-2.5-flash`
- **Orchestrator**: `gemini-2.5-flash`
- **Subagent**: `gemini-2.5-flash-lite`

### OpenAI Defaults
- **Single Agent**: `gpt-5`
- **Orchestrator**: `gpt-5`
- **Subagent**: `gpt-5-mini`

### Anthropic Defaults
- **Single Agent**: `claude-sonnet-4-20250514`
- **Orchestrator**: `claude-sonnet-4-20250514`
- **Subagent**: `claude-opus-4-1-20250805`

## Command Line Usage

### Auto Model Selection

Use `--model auto` to automatically select the provider-specific default:

```bash
# Will use provider defaults from vtagent.toml
vtagent --model auto

# Will use provider-specific defaults for multi-agent
vtagent --model auto --force-multi-agent
```

### Explicit Model Selection

Override configuration by specifying exact models:

```bash
# Use specific Gemini model
vtagent --model gemini-2.5-pro

# Use specific OpenAI model
vtagent --model gpt-5

# Use specific Anthropic model
vtagent --model claude-sonnet-4-20250514
```

## Environment Variables

Set appropriate API keys based on your provider:

```bash
# For Gemini
export GEMINI_API_KEY="your-gemini-key"
# or
export GOOGLE_API_KEY="your-google-key"

# For OpenAI
export OPENAI_API_KEY="your-openai-key"

# For Anthropic
export ANTHROPIC_API_KEY="your-anthropic-key"
```

## Example Configurations

### Gemini-Only Setup
```toml
[agent]
provider = "gemini"
api_key_env = "GEMINI_API_KEY"

[multi_agent]
enabled = true
provider = "gemini"
debug_mode = false
```

### OpenAI-Only Setup
```toml
[agent]
provider = "openai"
default_model = "gpt-5"
api_key_env = "OPENAI_API_KEY"

[multi_agent]
enabled = true
provider = "openai"
orchestrator_model = "gpt-5"
subagent_model = "gpt-5-mini"
debug_mode = false
```

### Anthropic-Only Setup
```toml
[agent]
provider = "anthropic"
default_model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"

[multi_agent]
enabled = true
provider = "anthropic"
orchestrator_model = "claude-sonnet-4-20250514"
subagent_model = "claude-opus-4-1-20250805"
debug_mode = false
```

### Mixed Provider Setup
```toml
[agent]
# Default provider for single agent
provider = "gemini"

[multi_agent]
enabled = true
# Different provider for multi-agent
provider = "openai"
orchestrator_model = "gpt-5"
subagent_model = "gpt-5-mini"
```

## Debug Mode

Enable debug mode to see provider and model selection:

```toml
[multi_agent]
debug_mode = true
```

Output will show:
```
[DEBUG] Multi-agent debug mode enabled
[DEBUG] Session ID: session_1234567890
[DEBUG] Provider: openai
[DEBUG] Orchestrator model: gpt-5
[DEBUG] Subagent model: gpt-5-mini
[DEBUG] Max concurrent subagents: 3
```

## Migration from Previous Versions

If you have existing configurations that only specify models:

### Before
```toml
[agent]
default_model = "gemini-2.5-flash"

[multi_agent]
orchestrator_model = "gemini-2.5-flash"
subagent_model = "gemini-2.5-flash-lite"
```

### After (Recommended)
```toml
[agent]
provider = "gemini"
# default_model is optional - will use provider defaults

[multi_agent]
provider = "gemini"
# models are optional - will use provider defaults
```

## Provider Selection Logic

1. **Explicit Model**: If you specify `--model model-name`, that exact model is used
2. **Auto Model**: If you use `--model auto` or don't specify a model:
   - Check `vtagent.toml` for provider configuration
   - Use provider-specific defaults for the selected mode (single/multi)
   - Fall back to global defaults if provider parsing fails

## Benefits

### For Developers
- **Flexibility**: Switch between providers easily
- **Cost Optimization**: Use efficient models for subagents, powerful models for orchestration
- **Provider Independence**: Not locked into a single AI provider

### For Organizations
- **Compliance**: Use approved AI providers based on policy
- **Cost Management**: Choose cost-effective providers and models
- **Performance**: Optimize for speed vs. capability based on use case

### For Research
- **Model Comparison**: Easily test different providers and models
- **Benchmarking**: Compare performance across providers
- **Provider Diversity**: Reduce dependency on single provider

## Troubleshooting

### Model Not Found
If you get "Invalid model identifier" errors:
1. Check that the model name is spelled correctly
2. Verify the model is supported by your provider
3. Use `vtagent --help` to see available models

### API Key Issues
If you get authentication errors:
1. Verify the correct environment variable is set
2. Check that your API key is valid
3. Ensure you have sufficient credits/quota

### Provider Parsing Errors
If provider configuration fails:
1. Check that provider is one of: `gemini`, `openai`, `anthropic`
2. Verify the configuration file syntax is correct
3. Use debug mode to see what's being parsed

## Best Practices

1. **Use Provider Defaults**: Let VTAgent choose optimal models for each provider
2. **Enable Debug Mode**: During setup to verify correct provider/model selection
3. **Set Environment Variables**: Use appropriate API key variables for each provider
4. **Test Configurations**: Verify each provider works before production use
5. **Document Choices**: Note why specific providers/models were chosen for your use case
