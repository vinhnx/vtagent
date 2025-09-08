# LLM Provider Migration Guide

## Current State: Gemini-Specific Implementation

The current `gemini.rs` module is **heavily provider-specific** with these limitations:

### 🔴 **Gemini-Specific Elements**
1. **Hardcoded API Endpoint**: `https://generativelanguage.googleapis.com/v1beta/models/`
2. **Gemini Authentication**: API key in URL query parameter
3. **Gemini Request Format**: `GenerateContentRequest` with Gemini field names
4. **Gemini Response Parsing**: Expects specific Gemini response structure
5. **Gemini Function Calling**: Uses Gemini's function calling modes

## Proposed Solution: Universal LLM Provider Architecture

### 🎯 **Architecture Overview**

```
llm/
├── provider.rs          # Universal LLM trait and types
├── factory.rs           # Provider factory and registry
├── client.rs            # Unified client interface
└── providers/
    ├── gemini.rs        # Gemini implementation
    ├── openai.rs        # OpenAI implementation
    └── anthropic.rs     # Anthropic implementation
```

### 🔧 **Key Components**

#### 1. Universal LLM Provider Trait
```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;
    fn supported_models(&self) -> Vec<String>;
    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError>;
}
```

#### 2. Universal Request/Response Types
```rust
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub model: String,
    // ... other universal parameters
}

pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: Option<Usage>,
    pub finish_reason: FinishReason,
}
```

#### 3. Provider Factory
```rust
pub struct LLMFactory {
    providers: HashMap<String, Box<dyn Fn(String) -> Box<dyn LLMProvider>>>,
}

// Auto-detect provider from model name
pub fn create_provider_for_model(model: &str, api_key: String) -> Result<Box<dyn LLMProvider>, LLMError>
```

## Migration Steps

### Phase 1: Create Universal Abstractions ✅
- [x] Define `LLMProvider` trait
- [x] Create universal request/response types
- [x] Implement provider factory

### Phase 2: Implement Provider Backends ✅
- [x] **GeminiProvider**: Converts universal format ↔ Gemini API
- [x] **OpenAIProvider**: Converts universal format ↔ OpenAI API  
- [x] **AnthropicProvider**: Converts universal format ↔ Anthropic API

### Phase 3: Create Unified Client ✅
- [x] `UnifiedLLMClient` that works with any provider
- [x] Auto-detection of provider from model name
- [x] Backward-compatible interface

### Phase 4: Migration Path
```rust
// OLD: Gemini-specific
let client = gemini::Client::new(api_key, model);
let response = client.generate_content(&request).await?;

// NEW: Universal
let client = UnifiedLLMClient::new(model, api_key)?;
let response = client.generate(messages, system_prompt).await?;
```

## Provider Compatibility Analysis

### ✅ **OpenAI Compatibility**
- **Authentication**: Bearer token in header ✅
- **Request Format**: Chat completions API ✅
- **Tool Calling**: Native function calling support ✅
- **Streaming**: Server-sent events ✅
- **Models**: GPT-4, GPT-3.5, etc. ✅

### ✅ **Anthropic Compatibility**  
- **Authentication**: x-api-key header ✅
- **Request Format**: Messages API ✅
- **Tool Calling**: Different format (needs adaptation) ⚠️
- **Streaming**: Server-sent events ✅
- **Models**: Claude 3.5, Claude 3, etc. ✅

### ✅ **Gemini Compatibility**
- **Authentication**: API key in URL ✅
- **Request Format**: GenerateContent API ✅
- **Tool Calling**: Function declarations ✅
- **Streaming**: Server-sent events ✅
- **Models**: Gemini 2.5, Gemini 1.5, etc. ✅

## Implementation Benefits

### 🚀 **Immediate Benefits**
1. **Provider Flexibility**: Switch between Gemini, OpenAI, Anthropic
2. **Model Flexibility**: Use any model from any provider
3. **Cost Optimization**: Choose cheapest/fastest model for each task
4. **Reliability**: Fallback to different providers

### 🔧 **Technical Benefits**
1. **Clean Abstractions**: Universal interfaces hide provider complexity
2. **Testability**: Mock providers for testing
3. **Extensibility**: Easy to add new providers
4. **Maintainability**: Provider-specific code isolated

### 📈 **Business Benefits**
1. **Vendor Independence**: Not locked into single provider
2. **Cost Control**: Compare pricing across providers
3. **Performance Optimization**: Use best model for each use case
4. **Risk Mitigation**: Reduce dependency on single provider

## Usage Examples

### Basic Text Generation
```rust
// Works with any provider/model
let client = UnifiedLLMClient::new("gpt-4o".to_string(), openai_key)?;
let response = client.generate(
    vec![Message::user("Explain quantum computing".to_string())],
    Some("You are a helpful physics teacher".to_string())
).await?;
```

### Tool Calling
```rust
let tools = vec![ToolDefinition {
    name: "get_weather".to_string(),
    description: "Get weather for a location".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        }
    })
}];

let response = client.generate_with_tools(messages, system_prompt, tools).await?;
```

### Provider Switching
```rust
// Easy to switch providers
let gemini_client = UnifiedLLMClient::new("gemini-2.5-flash".to_string(), gemini_key)?;
let openai_client = UnifiedLLMClient::new("gpt-4o".to_string(), openai_key)?;
let claude_client = UnifiedLLMClient::new("claude-3-5-sonnet-20241022".to_string(), anthropic_key)?;

// Same interface for all
let response = gemini_client.generate(messages.clone(), system.clone()).await?;
let response = openai_client.generate(messages.clone(), system.clone()).await?;
let response = claude_client.generate(messages, system).await?;
```

## Conclusion

The proposed universal LLM provider architecture transforms the Gemini-specific implementation into a flexible, extensible system that supports multiple AI providers while maintaining clean abstractions and backward compatibility. This enables VTAgent to leverage the best models from different providers and reduces vendor lock-in risks.
