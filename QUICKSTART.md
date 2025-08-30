# VTAGENT - Quick Start

The **simplest way** to run vtagent:

## Option 1: One-Line Setup (Recommended)

```bash
# 1. Set your API key
export GEMINI_API_KEY="your_key_here"

# 2. Run the agent
./run.sh
```

That's it! The script handles everything else automatically, including enabling advanced features like async file operations and real-time file diffs.

## Option 2: Manual Setup

```bash
# 1. Get your API key from https://aistudio.google.com/app/apikey
export GEMINI_API_KEY="your_key_here"

# 2. Build and run
cargo build --release
cargo run --release -- chat
```

## What Happens Next

1. **Welcome Screen** - You'll see the VTAGENT banner
2. **Interactive Chat** - Type your coding questions
3. **AI Assistance** - The agent will help with:
   - Code generation and editing
   - File operations
   - Project analysis
   - Multi-language support (Rust, Python, JavaScript, etc.)

## Example Usage

```
vtagent: Hello! How can I help you with coding today?

You: Create a Rust function to calculate fibonacci numbers

vtagent: I'll create a fibonacci function for you...

[Agent creates the file and shows you the result]
```

## Need Help?

- **API Key**: Get it from [Google AI Studio](https://aistudio.google.com/app/apikey)
- **Models**: Uses `gemini-2.5-flash-lite` by default (fastest)
- **Exit**: Press `Ctrl+C` to quit the chat

## Advanced Usage

```bash
# Fast debug build (recommended for development)
./run-debug.sh

# Debug build with run.sh
./run.sh debug

# Verbose mode (shows detailed logging)
cargo run -- chat-verbose

# With async file operations
cargo run -- --async-file-ops chat

# Analyze your workspace
cargo run -- analyze
```

Happy coding!
