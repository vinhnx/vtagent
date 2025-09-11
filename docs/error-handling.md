# Error Handling in VTAgent

VTAgent provides user-friendly error messages with ANSI formatting and actionable suggestions when connection errors occur with different providers.

## LMStudio Connection Errors

When VTAgent cannot connect to an LMStudio server, users will see a formatted error message like:

```
❌ Connection Error: LMStudio Server Not Found

Possible causes:
  • LMStudio is not running
  • LMStudio server is not configured correctly
  • Wrong base URL or port

How to fix:
  1. Start LMStudio
     • Launch LMStudio application
     • Start the local server
  2. Verify server configuration
     • Check that the server is running on the expected port (default: 1234)
     • Ensure the base URL is correct in your configuration
  3. Load a model
     • Load a model in LMStudio before using VTAgent
```

## Implementation Details

The error handling is implemented in the chat command handler in `src/cli/chat.rs`. When a connection test fails, the system:

1. Identifies the provider from the model configuration
2. Displays a provider-specific error message with ANSI formatting
3. Provides step-by-step instructions to fix the issue
4. Offers alternative solutions

The error messages use ANSI colors for better readability:
- Red bold for error titles
- Yellow bold for section headers
- Green bold for solution steps
- Cyan bold for alternative options
- Dimmed text for documentation references

This approach helps users quickly diagnose and resolve connection issues without needing to consult external documentation.