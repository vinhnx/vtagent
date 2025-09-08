#!/bin/bash

# VTAGENT - Simple Launch Script
# This script provides the easiest way to run vtagent

set -e

echo "VTAGENT - Research-preview Rust Coding Agent"
echo "=================================================="

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

# Build and run
echo "Building vtagent in release mode (this may take a few minutes)..."
echo "Tip: Use './run.sh debug' for faster builds during development"
echo ""

# Check if user wants debug build
if [[ "$1" == "debug" ]]; then
    echo "Using debug build for faster compilation..."
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
    cargo run -- --async-file-ops --show-file-diffs chat
else
    cargo build --release
    echo ""
    echo "Build complete!"
    echo ""
    echo "Starting vtagent chat with advanced features..."
    echo "  - Async file operations enabled for better performance"
    echo "  - Real-time file diffs enabled in chat"
    echo "  - Type your coding questions and requests"
    echo "  - Press Ctrl+C to exit"
    echo "  - The agent has access to file operations and coding tools"
    echo ""
    cargo run --release -- --async-file-ops --show-file-diffs chat
fi
