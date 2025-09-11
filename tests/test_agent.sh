#!/bin/bash

# Test script for VTAgent with LMStudio

echo "Testing VTAgent with LMStudio..."

# Make sure LMStudio is running
echo "Make sure LMStudio is running with a model loaded at http://localhost:1234"

# Test single agent mode
echo "Testing single agent mode..."
echo "hello" | timeout 10s cargo run -- chat

# Test multi-agent mode
echo "Testing multi-agent mode..."
echo "hello" | timeout 10s cargo run -- chat