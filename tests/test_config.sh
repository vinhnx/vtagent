#!/usr/bin/env bash

# Test script for VTAgent config loading from home directory

echo "Testing VTAgent configuration loading..."

# Check if ~/.vtagent directory exists
if [ -d "$HOME/.vtagent" ]; then
    echo "✓ Found ~/.vtagent directory"
    
    # Check if vtagent.toml exists in ~/.vtagent
    if [ -f "$HOME/.vtagent/vtagent.toml" ]; then
        echo "✓ Found ~/.vtagent/vtagent.toml"
        
        # Check if the file has content
        if [ -s "$HOME/.vtagent/vtagent.toml" ]; then
            echo "✓ Configuration file has content"
        else
            echo "⚠ Configuration file is empty"
        fi
    else
        echo "✗ ~/.vtagent/vtagent.toml not found"
    fi
else
    echo "✗ ~/.vtagent directory not found"
fi

echo "Test completed."