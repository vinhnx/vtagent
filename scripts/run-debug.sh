#!/bin/bash

# VTAGENT - Debug Mode Launch Script
# This script provides fast development builds

set -e

echo "VTAGENT - Debug Mode (Fast Build)"
echo "=================================="

# Check if API key is set
if [[ -z "$GEMINI_API_KEY" && -z "$GOOGLE_API_KEY" ]]; then
    echo "Error: API key not found!"
    echo ""
    echo "Please set one of these environment variables:"
    echo "  export GEMINI_API_KEY='your_gemini_api_key_here'"
    echo "  export GOOGLE_API_KEY='your_google_api_key_here'"
    echo ""
    echo "Get your API key from: https://aistudio.google.com/app/apikey"
    exit 1
fi

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "Error: Please run this script from the vtagent project root directory"
    exit 1
fi

# Build and run in debug mode (much faster)
echo "Building vtagent in debug mode..."
cargo build

echo ""
echo "Debug build complete!"
echo ""
echo "Starting vtagent chat with advanced features..."
echo "  - Async file operations enabled for better performance"
echo "  - Real-time file diffs enabled in chat"
echo "  - Type your coding questions and requests"
echo "  - Press Ctrl+C to exit"
echo "  - The agent has access to file operations and coding tools"
echo ""

# Run with advanced features enabled by default
cargo run --  --show-file-diffs --debug chat