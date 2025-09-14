# Development Scripts

This directory contains scripts to help with development, testing, and maintaining the vtagent codebase.

## Available Scripts

### `setup.sh` - Development Environment Setup

Sets up the complete development environment with all necessary tools.

```bash
# Basic setup
./scripts/setup.sh

# Setup with git hooks
./scripts/setup.sh --with-hooks

# Show help
./scripts/setup.sh --help
```

**What it does:**

- Checks Rust installation
- Updates Rust toolchain
- Installs rustfmt and clippy components
- Installs development tools (cargo-audit, cargo-outdated, etc.)
- Optionally sets up git hooks
- Verifies everything works

### `check.sh` - Code Quality Checks

Runs comprehensive code quality checks (same as CI pipeline).

```bash
# Run all checks
./scripts/check.sh

# Run specific checks
./scripts/check.sh fmt      # Format check only
./scripts/check.sh clippy   # Clippy check only
./scripts/check.sh test     # Tests only
./scripts/check.sh build    # Build only
./scripts/check.sh docs     # Documentation only

# Show help
./scripts/check.sh help
```

**Checks performed:**

- Code formatting (rustfmt)
- Linting (clippy)
- Build verification
- Test execution
- Documentation generation

### `release.sh` - Release Management

Creates releases for VTAgent using changelogithub integration.

```bash
# Create a specific version release
./scripts/release.sh 1.0.0

# Create patch release (increment patch version)
./scripts/release.sh --patch

# Create minor release (increment minor version)
./scripts/release.sh --minor

# Create major release (increment major version)
./scripts/release.sh --major

# Dry run to see what would happen
./scripts/release.sh --patch --dry-run

# Show help
./scripts/release.sh --help
```

**What it does:**

- Updates version in `Cargo.toml`
- Creates git tag with proper versioning
- Pushes tag to GitHub (triggers release workflow)
- GitHub Actions automatically generates changelog and creates release

**Prerequisites:**

- Must be on `main` branch
- Working tree must be clean
- Requires GitHub repository access

**Release Process:**

1. **Pre-flight checks**: Verifies branch and working tree state
2. **Version update**: Updates `Cargo.toml` with new version
3. **Git operations**: Commits version change, creates tag, pushes to GitHub
4. **Automated release**: GitHub Actions creates release with changelog using the official changelogithub action

**Recent Updates:**

- Fixed changelogithub CLI compatibility issues by switching to the official GitHub Action
- Added proper workflow permissions for release creation
- Improved error handling and debugging information

## Quick Start

For new developers:

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd vtagent
   ```

2. **Set up development environment**

   ```bash
   ./scripts/setup.sh --with-hooks
   ```

3. **Configure API keys (optional)**

   Create a `.env` file in the project root to store your API keys:
   ```bash
   # .env file
   GEMINI_API_KEY=your_gemini_api_key_here
   ANTHROPIC_API_KEY=your_anthropic_api_key_here
   OPENAI_API_KEY=your_openai_api_key_here
   ```

4. **Run code quality checks**

   ```bash
   ./scripts/check.sh
   ```

5. **Start developing!**

   ```bash
   cargo build
   cargo nextest run
   ```

## API Key Configuration

vtagent supports multiple ways to configure API keys, with the following priority:

1. **Environment variables** (highest priority) - Most secure
2. **.env file** - Convenient for development
3. **Configuration file** (vtagent.toml) - Least secure, but convenient

Example `.env` file:
```bash
GEMINI_API_KEY=your_gemini_api_key_here
ANTHROPIC_API_KEY=your_anthropic_api_key_here
OPENAI_API_KEY=your_openai_api_key_here
```

Example `vtagent.toml` configuration:
```toml
[agent]
gemini_api_key = "your_gemini_api_key_here"
anthropic_api_key = "your_anthropic_api_key_here"
openai_api_key = "your_openai_api_key_here"
```

## Integration with CI/CD

These scripts run the same checks as our GitHub Actions workflows:

- `ci.yml` - Main CI pipeline
- `code-quality.yml` - Code quality checks
- `development.yml` - Development workflow
- `nightly.yml` - Nightly builds

## Pre-commit Hooks

When you run `./scripts/setup.sh --with-hooks`, a pre-commit hook is created that will:

1. Check code formatting with rustfmt
2. Run clippy linting
3. Prevent commits if issues are found

The hook can be bypassed with `git commit --no-verify` if needed.

## Customization

You can modify these scripts to fit your development workflow:

- Add additional tools to `setup.sh`
- Modify check criteria in `check.sh`
- Customize git hooks for your team

## Troubleshooting

### Script permissions

```bash
chmod +x scripts/*.sh
```

### Rust not found

Make sure Rust is installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Tools installation fails

Some tools might require additional dependencies:

```bash
# For cargo-tarpaulin (code coverage)
sudo apt-get install libssl-dev pkg-config

# For cargo-udeps (unused dependencies)
rustup install nightly
```

## Related Documentation

- [CI/CD Guide](../docs/development/ci-cd.md)
- [Contributing Guide](../docs/development/README.md)
- [Code Quality Standards](../docs/project/README.md)