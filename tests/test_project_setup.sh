#!/usr/bin/env bash

# Test script for VTAgent project management

echo "Testing VTAgent project management..."

# Check if ~/.vtagent/projects directory exists
if [ -d "$HOME/.vtagent/projects" ]; then
    echo "✓ Found ~/.vtagent/projects directory"
else
    echo "✗ ~/.vtagent/projects directory not found"
    exit 1
fi

# Test creating a sample project structure
echo "Creating test project structure..."
mkdir -p "$HOME/.vtagent/projects/test-project/config"
mkdir -p "$HOME/.vtagent/projects/test-project/cache"
mkdir -p "$HOME/.vtagent/projects/test-project/embeddings"
mkdir -p "$HOME/.vtagent/projects/test-project/retrieval"

# Check if all directories were created
if [ -d "$HOME/.vtagent/projects/test-project/config" ]; then
    echo "✓ Config directory created"
else
    echo "✗ Config directory not created"
fi

if [ -d "$HOME/.vtagent/projects/test-project/cache" ]; then
    echo "✓ Cache directory created"
else
    echo "✗ Cache directory not created"
fi

if [ -d "$HOME/.vtagent/projects/test-project/embeddings" ]; then
    echo "✓ Embeddings directory created"
else
    echo "✗ Embeddings directory not created"
fi

if [ -d "$HOME/.vtagent/projects/test-project/retrieval" ]; then
    echo "✓ Retrieval directory created"
else
    echo "✗ Retrieval directory not created"
fi

# Create a simple .project metadata file
cat > "$HOME/.vtagent/projects/test-project/.project" << EOF
{
  "name": "test-project",
  "description": "Test project for VTAgent",
  "created_at": $(date +%s),
  "updated_at": $(date +%s),
  "root_path": "/tmp/test-project",
  "tags": ["test", "vtagent"]
}
EOF

if [ -f "$HOME/.vtagent/projects/test-project/.project" ]; then
    echo "✓ Project metadata file created"
else
    echo "✗ Project metadata file not created"
fi

echo "Test completed."