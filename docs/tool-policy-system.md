# Tool Policy System

The VTAgent tool policy system provides fine-grained control over which tools the agent can use, storing user preferences persistently to minimize repeated prompts while maintaining security.

## Overview

The tool policy system implements the following workflow:

1. **First Run**: All tools start with "prompt" policy - user is asked for approval
2. **User Choice**: User approves or denies each tool
3. **Persistent Storage**: Choice is saved to `~/.vtagent/tool-policy.json`
4. **Future Runs**: Agent respects stored choices without re-prompting
5. **Dynamic Updates**: New tools are automatically added as "prompt", removed tools are cleaned up

## Policy Types

### Allow
- Tool executes automatically without user confirmation
- Best for trusted, safe tools like `read_file`, `list_files`
- Provides seamless agent experience

### Prompt
- Tool prompts user for confirmation each time (default for new tools)
- Good for tools you want to control on a case-by-case basis
- Balances security with flexibility

### Deny
- Tool is never allowed to execute
- Perfect for dangerous tools or tools you never want the agent to use
- Provides maximum security

## Configuration File

The tool policy configuration is stored in `~/.vtagent/tool-policy.json`:

```json
{
  "version": 1,
  "available_tools": [
    "read_file",
    "write_file",
    "list_files",
    "run_terminal_cmd",
    "rp_search"
  ],
  "policies": {
    "read_file": "allow",
    "write_file": "prompt",
    "list_files": "allow",
    "run_terminal_cmd": "deny",
    "rp_search": "allow"
  }
}
```

## CLI Commands

### View Current Status
```bash
vtagent tool-policy status
```

Shows all tools and their current policies with color coding:
- 🟢 **ALLOW** - Green
- 🟡 **PROMPT** - Yellow  
- 🔴 **DENY** - Red

### Allow a Tool
```bash
vtagent tool-policy allow read_file
```

### Deny a Tool
```bash
vtagent tool-policy deny run_terminal_cmd
```

### Set Tool to Prompt
```bash
vtagent tool-policy prompt write_file
```

### Bulk Operations
```bash
# Allow all tools
vtagent tool-policy allow-all

# Deny all tools
vtagent tool-policy deny-all

# Reset all tools to prompt
vtagent tool-policy reset-all
```

## User Experience Flow

### First Time Using a Tool

```
Tool Permission Request: write_file
The agent wants to use the 'write_file' tool.

Your choice will be remembered for future runs.
You can change this later via configuration or CLI flags.

Allow the agent to use 'write_file'? [y/N] y
✓ Approved: 'write_file' tool will be allowed in future runs
```

### Subsequent Uses

If approved, the tool executes silently. If denied, execution is blocked with a clear message.

## Integration with Agent

The tool policy system integrates seamlessly with the agent's tool execution:

```rust
// Before executing any tool
if !policy_manager.should_execute_tool(tool_name)? {
    return Err(anyhow!("Tool '{}' execution denied by policy", tool_name));
}

// Tool executes normally
let result = execute_tool(tool_name, args).await?;
```

## Dynamic Tool Management

### Adding New Tools

When new tools are added to the agent:
1. They are automatically added to the configuration as "prompt"
2. User will be asked for permission on first use
3. Choice is remembered for future runs

### Removing Tools

When tools are removed from the agent:
1. They are automatically removed from the configuration
2. No orphaned entries remain in the policy file
3. Configuration stays clean and up-to-date

## Security Benefits

### User Control
- Users have complete control over which tools can execute
- No tool can run without explicit user permission (initially)
- Policies can be changed at any time

### Audit Trail
- All policy decisions are stored persistently
- Clear visibility into which tools are allowed/denied
- Easy to review and modify security posture

### Defense in Depth
- Works alongside existing command allow/deny lists
- Provides tool-level granularity beyond command-level controls
- Complements other security measures

## Best Practices

### Initial Setup
1. Start with default "prompt" for all tools
2. Allow safe, read-only tools like `read_file`, `list_files`
3. Be cautious with write operations and command execution
4. Review and adjust policies based on usage patterns

### Ongoing Management
1. Regularly review tool policies with `vtagent tool-policy status`
2. Adjust policies based on trust level and usage frequency
3. Use "prompt" for tools you want to control situationally
4. Keep dangerous tools on "deny" unless absolutely needed

### Team Environments
1. Document team policies for tool usage
2. Consider standardized policy configurations
3. Share policy files for consistent security posture
4. Regular security reviews of tool permissions

## Troubleshooting

### Tool Execution Blocked
```
Error: Tool 'run_terminal_cmd' execution denied by policy
```

**Solution**: Check policy with `vtagent tool-policy status` and adjust if needed:
```bash
vtagent tool-policy allow run_terminal_cmd
```

### Configuration File Issues
If the configuration file becomes corrupted:
1. Delete `~/.vtagent/tool-policy.json`
2. Restart the agent - it will recreate with defaults
3. Reconfigure your preferred policies

### Missing Tools in Configuration
If tools don't appear in the configuration:
1. The agent automatically adds new tools as "prompt"
2. Use the agent once to trigger tool discovery
3. Check status to see newly added tools

## Implementation Details

### File Location
- **Path**: `~/.vtagent/tool-policy.json`
- **Format**: JSON with version for future compatibility
- **Permissions**: User read/write only

### Thread Safety
- Configuration is loaded once at startup
- Changes are immediately persisted to disk
- No concurrent modification issues

### Performance
- Minimal overhead - simple HashMap lookup
- Configuration cached in memory
- File I/O only on policy changes

### Error Handling
- Graceful fallback to "prompt" for missing policies
- Clear error messages for configuration issues
- Automatic recovery from corrupted files

## Future Enhancements

### Planned Features
- Time-based policies (allow tool for X minutes)
- Context-aware policies (allow in certain directories)
- Policy templates for common use cases
- Integration with external policy management systems

### Advanced Configuration
- Per-project policy overrides
- Environment-specific policies (dev/prod)
- Policy inheritance and composition
- Audit logging of all tool executions

The tool policy system provides a robust foundation for secure, user-controlled agent tool execution while maintaining a smooth user experience through intelligent defaults and persistent storage.
