# VT Code Distribution Setup

This document outlines the complete distribution setup for VT Code across multiple package managers and platforms.

## Distribution Channels

### 1. Cargo (crates.io)

-   **Primary Rust package repository**
-   **Location**: `https://crates.io/crates/vtcode`
-   **Workflow**: `.github/workflows/publish-crates.yml`
-   **Metadata**: Added to `Cargo.toml` and `vtcode-core/Cargo.toml`

### 2. Homebrew (macOS)

-   **Formula**: `homebrew/vtcode.rb`
-   **Installation**: `brew install vinhnx/tap/vtcode`
-   **Binaries**: Downloaded from GitHub Releases

### 3. npm (Cross-platform)

-   **Package**: `@vinhnx/vtcode` (when published)
-   **Installation**: `npm install -g vtcode`
-   **Structure**: `npm/` directory with postinstall script

### 4. GitHub Releases

-   **Binaries**: Pre-built for multiple platforms
-   **Workflow**: `.github/workflows/build-release.yml`
-   **Platforms**: Linux x64, macOS x64/ARM64, Windows x64

## File Structure

```
vtcode/
├── Cargo.toml                    # Main crate metadata
├── vtcode-core/
│   └── Cargo.toml               # Core library metadata
├── homebrew/
│   └── vtcode.rb               # Homebrew formula
├── npm/
│   ├── package.json            # npm package config
│   ├── index.js               # Main entry point
│   ├── bin/
│   │   └── vtcode            # Executable wrapper
│   └── scripts/
│       ├── postinstall.js     # Binary download script
│       └── preuninstall.js    # Cleanup script
├── .github/workflows/
│   ├── publish-crates.yml     # Cargo publishing
│   ├── build-release.yml      # Binary builds
│   └── release.yml           # Release creation
└── scripts/
    ├── release.sh            # Release management
    └── test-distribution.sh  # Distribution validation
```

## Release Process

1. **Create Release**: Use `./scripts/release.sh` to bump version and create git tag
2. **Build Binaries**: GitHub Actions automatically builds binaries for all platforms
3. **Publish to Cargo**: Automatically publishes to crates.io
4. **Manual Steps**:
    - Publish npm package: `cd npm && npm publish`
    - Update Homebrew formula with correct SHA256 hashes
    - Create Homebrew tap if needed

## Validation

Run `./scripts/test-distribution.sh` to validate the entire setup before releasing.

## Secrets Required

-   `CRATES_IO_TOKEN`: For publishing to crates.io
-   `GITHUB_TOKEN`: Automatically provided by GitHub Actions

## Next Steps

1. Create a test release to validate the pipeline
2. Set up npm publishing (requires npm account)
3. Create Homebrew tap repository
4. Update documentation with final installation URLs
