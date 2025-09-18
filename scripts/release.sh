#!/bin/bash

# VTCode Release Script
# Enhanced with cargo-release for better release management

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_distribution() {
    echo -e "${PURPLE}DISTRIBUTION: $1${NC}"
}

# Function to check if we're on main branch
check_branch() {
    local current_branch=$(git branch --show-current)
    if [ "$current_branch" != "main" ]; then
        print_error "You must be on the main branch to create a release"
        print_info "Current branch: $current_branch"
        print_info "Please switch to main branch: git checkout main"
        exit 1
    fi
}

# Function to check if working tree is clean
check_clean_tree() {
    if [ -n "$(git status --porcelain)" ]; then
        print_error "Working tree is not clean. Please commit or stash your changes."
        git status --short
        exit 1
    fi
}

# Function to check if cargo-release is available
check_cargo_release() {
    if ! command -v cargo-release &> /dev/null; then
        print_error "cargo-release is not installed"
        print_info "Install it with: cargo install cargo-release"
        exit 1
    fi
    print_success "cargo-release is available"
}

# Function to check Cargo authentication
check_cargo_auth() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not available"
        return 1
    fi

    # Check if credentials file exists
    local credentials_file="$HOME/.cargo/credentials.toml"
    if [[ ! -f "$credentials_file" ]]; then
        print_warning "Not logged in to crates.io"
        print_info "Run: cargo login"
        print_info "Get your API token from: https://crates.io/me"
        return 1
    fi

    # Check if credentials file has content (not empty)
    if [[ ! -s "$credentials_file" ]]; then
        print_warning "Cargo credentials file is empty"
        print_info "Run: cargo login"
        print_info "Get your API token from: https://crates.io/me"
        return 1
    fi

    print_success "Cargo authentication verified"
}

# Function to check Homebrew setup
check_homebrew_setup() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_info "Not on macOS - skipping Homebrew checks"
        return 0
    fi

    if ! command -v brew &> /dev/null; then
        print_warning "Homebrew is not available"
        return 1
    fi

    print_success "Homebrew is available"
}

# Function to trigger docs.rs rebuild
trigger_docs_rs_rebuild() {
    local dry_run=$1

    print_distribution "Triggering docs.rs rebuild..."

    if [[ "$dry_run" == "true" ]]; then
        print_info "Dry run - would trigger docs.rs rebuild for vtcode and vtcode-core"
        return 0
    fi

    # Check if CRATES_IO_TOKEN is available
    if [[ -z "$CRATES_IO_TOKEN" ]]; then
        print_warning "CRATES_IO_TOKEN not set - skipping docs.rs rebuild trigger"
        print_info "Note: docs.rs rebuild is usually automatic after crates.io publishing"
        return 0
    fi

    # Trigger docs.rs rebuild for vtcode-core
    print_info "Triggering docs.rs rebuild for vtcode-core..."
    if curl -X POST "https://docs.rs/crate/vtcode-core/latest/builds" \
             -H "Authorization: Bearer $CRATES_IO_TOKEN" \
             -H "Content-Type: application/json" \
             --silent --output /dev/null; then
        print_success "Triggered docs.rs rebuild for vtcode-core"
    else
        print_warning "Failed to trigger docs.rs rebuild for vtcode-core (this is usually automatic)"
    fi

    # Trigger docs.rs rebuild for vtcode
    print_info "Triggering docs.rs rebuild for vtcode..."
    if curl -X POST "https://docs.rs/crate/vtcode/latest/builds" \
             -H "Authorization: Bearer $CRATES_IO_TOKEN" \
             -H "Content-Type: application/json" \
             --silent --output /dev/null; then
        print_success "Triggered docs.rs rebuild for vtcode"
    else
        print_warning "Failed to trigger docs.rs rebuild for vtcode (this is usually automatic)"
    fi

    print_info "Note: docs.rs rebuild is usually automatic after crates.io publishing"
}

# Function to update Homebrew formula
update_homebrew_formula() {
    local version=$1

    if [[ ! -f "homebrew/vtcode.rb" ]]; then
        print_warning "Homebrew formula not found - skipping Homebrew update"
        return 0
    fi

    print_distribution "Updating Homebrew formula..."

    # Use the automated update script if available
    if [[ -f "scripts/update-homebrew-formula.sh" ]]; then
        print_info "Using automated Homebrew formula update script..."
        if ./scripts/update-homebrew-formula.sh "$version" "vinhnx/homebrew-tap"; then
            print_success "Homebrew formula updated automatically"
            return 0
        else
            print_warning "Automated update failed, falling back to manual instructions"
        fi
    fi

    # Fallback to manual instructions
    print_info "Homebrew formula needs manual update:"
    print_info "1. Update version in homebrew/vtcode.rb to $version"
    print_info "2. Update SHA256 hashes for new binaries"
    print_info "3. Commit and push changes to homebrew tap repository"
    print_info "4. Users can then run: brew install vinhnx/tap/vtcode"
}

# Function to show usage
show_usage() {
    cat << EOF
VTCode Release Script with cargo-release integration

USAGE:
    $0 [OPTIONS] [LEVEL|VERSION]

ARGUMENTS:
    LEVEL|VERSION    Either bump by LEVEL or set the VERSION for all packages
                     Levels: major, minor, patch
                     Version: e.g., 1.0.0, 1.2.3

OPTIONS:
    -h, --help          Show this help message
    -d, --dry-run       Show what would be done without making changes
    --no-publish        Skip publishing to crates.io
    --no-homebrew       Skip Homebrew formula update
    --execute           Actually perform the release (required for execution)

EXAMPLES:
    $0 --dry-run patch                    # Show what patch release would do
    $0 --execute patch                    # Create patch release
    $0 --execute --no-publish 1.0.0       # Set version without publishing
    $0 --execute --no-homebrew minor      # Create minor release, skip Homebrew

DISTRIBUTION CHANNELS:
    - crates.io: Rust package registry (via cargo-release)
    - docs.rs: Automatic API documentation
    - Homebrew: macOS package manager
    - GitHub: Automatic tag and push (via cargo-release)

SETUP REQUIREMENTS:
    1. Install cargo-release: cargo install cargo-release
    2. Cargo: Run 'cargo login' with your crates.io API token
    3. Git: Ensure you're on main branch with clean working tree
    4. Homebrew: Set up tap repository for macOS distribution (optional)

EOF
}

# Main function
main() {
    local version=""
    local increment_type=""
    local dry_run=false
    local execute=false
    local no_publish=false
    local no_homebrew=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -d|--dry-run)
                dry_run=true
                shift
                ;;
            --execute)
                execute=true
                shift
                ;;
            --no-publish)
                no_publish=true
                shift
                ;;
            --no-homebrew)
                no_homebrew=true
                shift
                ;;
            -*)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
            *)
                if [ -n "$version" ]; then
                    print_error "Multiple versions specified"
                    exit 1
                fi
                version=$1
                shift
                ;;
        esac
    done

    # Validate arguments
    if [ -z "$version" ] && [ "$dry_run" = false ]; then
        print_error "Must specify version or use --dry-run"
        show_usage
        exit 1
    fi

    if [ "$dry_run" = true ] && [ "$execute" = true ]; then
        print_error "Cannot use both --dry-run and --execute"
        exit 1
    fi

    if [ "$execute" = false ] && [ "$dry_run" = false ]; then
        print_error "Must specify either --execute or --dry-run"
        show_usage
        exit 1
    fi

    # Pre-flight checks
    print_info "Running pre-flight checks..."
    check_branch
    check_clean_tree
    check_cargo_release

    if [[ "$no_publish" != "true" ]]; then
        check_cargo_auth
    fi

    if [[ "$no_homebrew" != "true" ]]; then
        check_homebrew_setup
    fi

    # Build cargo-release command
    local cargo_release_cmd="cargo release"

    if [ "$dry_run" = true ]; then
        print_warning "DRY RUN - No changes will be made"
        cargo_release_cmd="$cargo_release_cmd --dry-run"
    fi

    if [ "$execute" = true ]; then
        cargo_release_cmd="$cargo_release_cmd --execute --no-confirm"
    fi

    if [ "$no_publish" = true ]; then
        cargo_release_cmd="$cargo_release_cmd --no-publish"
    fi

    # Add version/level
    if [ -n "$version" ]; then
        cargo_release_cmd="$cargo_release_cmd $version"
    fi

    # Show what will be done
    echo
    print_info "Will execute: $cargo_release_cmd"
    echo

    if [ "$dry_run" = true ]; then
        echo "This will show what cargo-release would do without making changes."
    else
        echo "This will perform the actual release using cargo-release."
        if [[ "$no_publish" != "true" ]]; then
            echo "  - Publish to crates.io"
        fi
        echo "  - Create git commit and tag"
        echo "  - Push to GitHub"
        if [[ "$no_homebrew" != "true" ]]; then
            echo "  - Update Homebrew formula"
        fi
    fi

    if [ "$execute" = true ]; then
        echo
        read -p "Are you sure you want to continue? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Release cancelled"
            exit 0
        fi
    fi

    # Execute cargo-release
    print_info "Starting release process with cargo-release..."
    if ! eval "$cargo_release_cmd"; then
        print_error "cargo-release failed"
        print_info "Check the output above for details"
        print_info "If the repository is in an inconsistent state, you may need to:"
        print_info "  git reset --hard HEAD~1  # Reset last commit"
        print_info "  git tag -d <tag-name>    # Delete created tag"
        exit 1
    fi

    if [ "$execute" = true ]; then
        print_success "cargo-release completed successfully!"

        # Additional post-release tasks
        if [[ "$no_publish" != "true" ]]; then
            trigger_docs_rs_rebuild false
        fi

        if [[ "$no_homebrew" != "true" ]]; then
            # Get the released version
            local released_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
            update_homebrew_formula "$released_version"
        fi

        print_success "Release $version completed successfully!"
        print_info "Distribution Summary:"
        if [[ "$no_publish" != "true" ]]; then
            print_info "  - Published to crates.io: https://crates.io/crates/vtcode"
            print_info "  - docs.rs updated: https://docs.rs/vtcode"
        fi
        if [[ "$no_homebrew" != "true" ]]; then
            print_info "  - Homebrew formula updated (manual step required)"
        fi
        print_info "  - GitHub Release: https://github.com/vinhnx/vtcode/releases/tag/v$version"
        print_info "  - Check https://github.com/vinhnx/vtcode/actions for CI status"
    fi
}

# Run main function
main "$@"
