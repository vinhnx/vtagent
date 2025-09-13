# VTAgent Provider Guides

## Overview

VTAgent supports multiple AI providers, each with their own setup process and capabilities. This guide covers the setup and usage of all supported providers.

## Supported Providers

### 1. Gemini (Google)
**Provider ID**: `gemini`
**Best For**: Balanced performance, Google's latest models
**Setup**: [Google AI Studio](https://aistudio.google.com/app/apikey)

#### Quick Setup
```toml
[agent]
provider = "gemini"
default_model = "gemini-2.5-flash"
api_key_env = "GEMINI_API_KEY"
```

#### Available Models
- `gemini-2.5-flash` - Fast, efficient model
- `gemini-2.5-pro` - High-performance model
- `gemini-1.5-flash` - Previous generation fast model
- `gemini-1.5-pro` - Previous generation pro model

### 2. OpenAI
**Provider ID**: `openai`
**Best For**: Industry standard, reliable performance
**Setup**: [OpenAI Platform](https://platform.openai.com/api-keys)

#### Quick Setup
```toml
[agent]
provider = "openai"
default_model = "gpt-5"
api_key_env = "OPENAI_API_KEY"
```

#### Available Models
- `gpt-5` - Latest GPT-4 optimized
- `gpt-5-mini` - Fast, cost-effective GPT-4
- `gpt-4-turbo` - Previous generation turbo
- `gpt-3.5-turbo` - Fast, economical model

### 3. Anthropic (Claude)
**Provider ID**: `anthropic`
**Best For**: Safety-focused, high-quality responses
**Setup**: [Anthropic Console](https://console.anthropic.com/)

#### Quick Setup
```toml
[agent]
provider = "anthropic"
default_model = "claude-3-5-sonnet-20241022"
api_key_env = "ANTHROPIC_API_KEY"
```

#### Available Models
- `claude-3-5-sonnet-20241022` - Latest Claude 3.5 Sonnet
- `claude-3-5-haiku-20241022` - Fast, efficient Claude 3.5
- `claude-3-opus-20240229` - Most capable Claude 3
- `claude-3-sonnet-20240229` - Balanced Claude 3
- `claude-3-haiku-20240307` - Fast Claude 3

### 4. Groq
**Provider ID**: `groq`
**Best For**: Ultra-fast inference, real-time applications
**Setup**: [Groq Console](https://console.groq.com/)

#### Quick Setup
```toml
[agent]
provider = "groq"
default_model = "llama-3.1-70b-versatile"
api_key_env = "GROQ_API_KEY"
```

#### Available Models
- `llama-3.1-405b-reasoning` - Largest Llama model, reasoning-focused
- `llama-3.1-70b-versatile` - Balanced performance and speed
- `llama-3.1-8b-instant` - Fastest Llama model
- `mixtral-8x7b-32768` - Mixture of experts model
- `gemma2-9b-it` - Google's Gemma 2 model

### 5. OpenRouter
**Provider ID**: `openrouter`
**Best For**: Access to multiple providers through single API
**Setup**: [OpenRouter](https://openrouter.ai/)

#### Quick Setup
```toml
[agent]
provider = "openrouter"
default_model = "qwen/qwen3-next-80b-a3b-instruct"
api_key_env = "OPENROUTER_API_KEY"
```

### 6. LMStudio
**Provider ID**: `lmstudio`
**Best For**: Local AI models, privacy, offline usage
**Setup**: [LMStudio](https://lmstudio.ai/)

#### Quick Setup
```toml
[agent]
provider = "lmstudio"
default_model = "local-model"
# No API key required for local models
```

#### Available Models
- `local-model` - Generic local model
- `qwen/qwen3-1.7b` - Qwen 3 1.7B model
- `mistral-7b-instruct` - Mistral 7B Instruct
- `llama-2-7b-chat` - Llama 2 7B Chat
- `llama-2-13b-chat` - Llama 2 13B Chat
- `codellama-7b-instruct` - CodeLlama 7B Instruct
- `codellama-13b-instruct` - CodeLlama 13B Instruct

## Environment Variables

Set the appropriate environment variable for your chosen provider:

```bash
# Google Gemini
export GEMINI_API_KEY="your_gemini_api_key"

# OpenAI
export OPENAI_API_KEY="your_openai_api_key"

# Anthropic
export ANTHROPIC_API_KEY="your_anthropic_api_key"

# Groq
export GROQ_API_KEY="your_groq_api_key"

# OpenRouter
export OPENROUTER_API_KEY="your_openrouter_api_key"

# LMStudio (no API key needed for local models)
# Just ensure LMStudio is running on localhost:1234
```

## Configuration Examples

### Basic Single Provider Setup
```toml
[agent]
provider = "gemini"
default_model = "gemini-2.5-flash"
api_key_env = "GEMINI_API_KEY"
```

### Multi-Agent with Different Providers
```toml
[multi_agent]
enabled = true
execution_mode = "auto"

# Orchestrator uses high-capability model
orchestrator_model = "gemini-2.5-pro"
orchestrator_provider = "gemini"

# Subagents use fast models
subagent_model = "llama-3.1-8b-instant"
subagent_provider = "groq"
```

### Local Development with LMStudio
```toml
[agent]
provider = "lmstudio"
default_model = "qwen/qwen3-1.7b"
# No api_key_env needed for local models
```

## Testing Your Setup

Test your provider setup with these commands:

```bash
# Basic connectivity test
vtagent ask "Hello, how are you?"

# Test with specific model
vtagent ask "Explain this code" --model "claude-3-5-sonnet-20241022"

# Test multi-agent mode (if enabled)
vtagent chat "Help me refactor this function" --multi-agent
```

## Troubleshooting

### Common Issues

#### API Key Not Found
```
Error: API key not found for provider
```
**Solution**: Ensure the environment variable is set correctly
```bash
echo $GEMINI_API_KEY  # Should show your API key
```

#### Model Not Available
```
Error: Model not available for provider
```
**Solution**: Check that the model name is correct and available for your provider

#### Rate Limit Exceeded
```
Error: Rate limit exceeded
```
**Solution**: Wait a few minutes, or upgrade your API plan

#### LMStudio Connection Failed
```
Error: Connection refused
```
**Solution**: Ensure LMStudio is running and accessible at `http://localhost:1234`

### Provider-Specific Issues

#### Groq: Model Loading
If Groq models are slow to respond initially, this is normal - their LPU hardware needs a moment to load the model.

#### OpenRouter: Credit Balance
Monitor your OpenRouter credit balance, as you'll only pay for what you use.

#### LMStudio: Model Compatibility
Ensure your chosen model is compatible with LMStudio's OpenAI-compatible API format.

## Performance Comparison

| Provider | Speed | Cost | Quality | Best Use Case |
|----------|-------|------|---------|---------------|
| **Groq** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | Real-time applications |
| **Gemini** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | Balanced workloads |
| **OpenAI** | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | High-quality responses |
| **Anthropic** | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ | Safety-critical applications |
| **OpenRouter** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ | Multi-provider access |
| **LMStudio** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | Local/offline usage |

## Advanced Configuration

### Custom Base URLs
```toml
[agent]
provider = "openai"
base_url = "https://your-custom-endpoint.com/v1"
```

### Provider-Specific Settings
```toml
[groq]
# Groq-specific settings
temperature = 0.7
max_tokens = 4096

[lmstudio]
# LMStudio-specific settings
base_url = "http://localhost:1234/v1"
timeout_seconds = 300
```

### Model Fallbacks
```toml
[agent]
provider = "groq"
default_model = "llama-3.1-70b-versatile"
fallback_models = ["llama-3.1-8b-instant", "mixtral-8x7b-32768"]
```

## Migration Between Providers

### Switching from One Provider to Another

1. **Update Configuration**:
   ```toml
   [agent]
   provider = "new_provider"  # e.g., "anthropic"
   default_model = "new_model"  # e.g., "claude-3-5-sonnet-20241022"
   api_key_env = "NEW_API_KEY"  # e.g., "ANTHROPIC_API_KEY"
   ```

2. **Set New Environment Variable**:
   ```bash
   export ANTHROPIC_API_KEY="your_anthropic_key"
   ```

3. **Test the New Setup**:
   ```bash
   vtagent ask "Test message with new provider"
   ```

### Multi-Provider Setup
```toml
[multi_agent]
enabled = true

# Different providers for different roles
orchestrator_provider = "anthropic"
orchestrator_model = "claude-3-5-sonnet-20241022"

subagent_provider = "groq"
subagent_model = "llama-3.1-8b-instant"
```

## Best Practices

### Cost Optimization
- Use smaller/faster models (like `llama-3.1-8b-instant`) for simple tasks
- Use larger models (like `claude-3-5-sonnet`) only when needed
- Consider OpenRouter for access to multiple providers without separate API keys

### Performance Optimization
- Use Groq for speed-critical applications
- Use LMStudio for offline/local development
- Use Gemini or OpenAI for balanced performance

### Reliability
- Set up model fallbacks for critical applications
- Monitor API rate limits and usage
- Use multi-agent mode for complex tasks requiring multiple specialized models

---

*For technical implementation details of each provider, see [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)*
