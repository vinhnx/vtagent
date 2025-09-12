# VTAgent Dot-Folder Project Management System

## Overview

VTAgent now supports a sophisticated dot-folder-based project management system that organizes project-specific data in `~/.vtagent/projects/<project-name>/`. This structure enables per-project isolation, efficient caching, semantic search capabilities, and improved agent performance.

## Directory Structure

The system creates the following directory structure in `~/.vtagent/projects/<project-name>/`:

```
~/.vtagent/projects/
└── <project-name>/
    ├── .project                 # Project metadata file
    ├── config/                  # Project-specific configuration files
    ├── cache/                   # Temporary data and cached computations
    ├── embeddings/              # Vector embeddings for semantic search
    └── retrieval/               # Indexed knowledge bases for RAG
```

### Directory Details

- **`.project`**: JSON file containing project metadata including name, description, creation timestamp, and root path
- **`config/`**: Project-specific configuration files that override global settings
- **`cache/`**: Temporary data storage with TTL (time-to-live) expiration and automatic invalidation
- **`embeddings/`**: Vector embeddings generated from project files for semantic search
- **`retrieval/`**: Indexed knowledge bases used for Retrieval-Augmented Generation (RAG)

## Project Identification

Projects are identified using the following priority:

1. **`.project` file**: If a `.project` file exists in the workspace root, its contents determine the project name
2. **Directory name**: If no `.project` file exists, the current directory name is used as the project name

## Configuration Loading Priority

Configuration files are loaded with the following priority (highest to lowest):

1. `./vtagent.toml` (workspace root)
2. `./.vtagent/vtagent.toml` (workspace .vtagent directory)
3. `~/.vtagent/vtagent.toml` (global configuration)
4. `~/.vtagent/projects/<project-name>/config/vtagent.toml` (project-specific)
5. Built-in defaults

## Caching System

The caching system provides:

- **TTL Support**: Automatic expiration of cached data
- **Invalidation**: Manual cache clearing capabilities
- **Statistics**: Cache usage and performance metrics
- **Automatic Cleanup**: Regular cleaning of expired entries

### Cache Entry Structure

```json
{
  "data": "...",           // Cached data
  "created_at": 1234567890, // Creation timestamp (Unix epoch)
  "ttl_seconds": 3600       // Time-to-live in seconds (optional)
}
```

## Embedding and Retrieval System

The embedding system enables semantic search and Retrieval-Augmented Generation (RAG):

- **File Indexing**: Automatic indexing of project files
- **Content-Aware Embeddings**: Different embedding strategies based on content type
- **Semantic Search**: Find relevant content using natural language queries
- **Similarity Matching**: Cosine similarity for finding related content

### Supported Content Types

- **Code files**: `.rs`, `.py`, `.js`, `.ts`, `.java`, `.cpp`, `.c`, `.go`
- **Documentation**: `.md`, `.txt`, `.doc`, `.docx`
- **Configuration**: `.toml`, `.json`, `.yaml`, `.yml`
- **Web files**: `.html`, `.css`
- **Other files**: All other file types

## CLI Commands

### Initialize Project Structure

```bash
vtagent init-project [--name PROJECT_NAME] [--force] [--migrate]
```

Options:
- `--name`: Specify project name (defaults to current directory name)
- `--force`: Overwrite existing project structure
- `--migrate`: Migrate existing config/cache files

### Configuration Management

```bash
vtagent config [--global] [--output PATH]
```

Options:
- `--global`: Create configuration in `~/.vtagent/vtagent.toml`
- `--output`: Specify custom output path

## Migration Logic

When using `vtagent init-project --migrate`, the system automatically detects and migrates:

- `./vtagent.toml` → `~/.vtagent/projects/<project-name>/config/vtagent.toml`
- `./.vtagent/*` → Appropriate project subdirectories
- `./cache/` → `~/.vtagent/projects/<project-name>/cache/`
- `./config/` → `~/.vtagent/projects/<project-name>/config/`

## Launch-Time Integration

On agent launch, the system automatically:

1. **Identifies the current project** using the identification mechanism
2. **Initializes project-specific systems** (cache, embeddings, retrieval)
3. **Cleans expired cache entries** to maintain optimal performance
4. **Loads project context** including README files and metadata
5. **Prepares semantic search capabilities** for RAG-enhanced responses

## Security and Permissions

The system respects file system permissions and:

- Creates directories with secure default permissions
- Preserves original file permissions during migration
- Never overwrites files without explicit user confirmation
- Provides backup options before migration

## Cross-Platform Support

The system works across platforms:

- **Unix/Mac**: Uses `~` for home directory expansion
- **Windows**: Uses `%USERPROFILE%` for home directory detection
- **Path Separators**: Automatically handles platform-specific path separators

## Extensibility

The system is designed to be extensible:

- **Plugin Architecture**: Support for adding new project types
- **Custom Content Types**: Easy addition of new file type handlers
- **Configuration Hooks**: Extension points for custom initialization
- **Event System**: Notification system for project lifecycle events

## Performance Considerations

- **Lazy Loading**: Components are only initialized when needed
- **Memory Efficient**: Caching with automatic cleanup prevents memory bloat
- **Parallel Processing**: File indexing and embedding generation can be parallelized
- **Incremental Updates**: Only changed files are re-indexed

## Example Usage

### Setting up a new project:

```bash
cd /path/to/my/project
vtagent init-project --migrate
```

This creates:
```
~/.vtagent/projects/my-project/
├── .project
├── config/
├── cache/
├── embeddings/
└── retrieval/
```

### Using project-specific configuration:

Create `~/.vtagent/projects/my-project/config/vtagent.toml`:

```toml
[agent]
default_model = "gpt-4"
max_conversation_turns = 200

[security]
human_in_the_loop = true
```

## Best Practices

1. **Project Isolation**: Keep project-specific settings in the project directory
2. **Cache Management**: Regularly clean expired cache entries
3. **Embedding Updates**: Re-index files after significant changes
4. **Backup Strategy**: Maintain backups of important configuration files
5. **Version Control**: Consider version controlling project metadata files

## Troubleshooting

### Common Issues

1. **Permission Errors**: Ensure write access to `~/.vtagent/projects/`
2. **Migration Failures**: Check file permissions and disk space
3. **Cache Corruption**: Use `cache.clear()` to reset corrupted cache
4. **Embedding Issues**: Re-index files to regenerate embeddings

### Diagnostic Commands

```bash
# Check project structure
ls -la ~/.vtagent/projects/<project-name>/

# View project metadata
cat ~/.vtagent/projects/<project-name>/.project

# Check cache statistics
# (Available through programmatic API)
```

## Future Enhancements

Planned improvements include:

- **Cloud Sync**: Synchronize project data across devices
- **Advanced RAG**: Multi-modal embeddings and cross-project search
- **Performance Analytics**: Detailed performance monitoring and optimization
- **Integration APIs**: Better integration with popular development tools