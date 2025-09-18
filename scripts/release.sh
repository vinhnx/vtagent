#!/bin/bash

# VTCode Release Script
# This script helps create releases for VTCode with support for multiple distribution channels

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

# Function to check Cargo authentication
check_cargo_auth() {
    if ! cargo login --help &> /dev/null; then
        print_error "Cargo is not available"
        return 1
    fi

    # Check if user is logged in to crates.io
    if ! cargo publish --dry-run &> /dev/null; then
        print_warning "Not logged in to crates.io"
        print_info "Run: cargo login"
        print_info "Get your API token from: https://crates.io/me"
        return 1
    fi

    print_success "Cargo authentication verified"
}

# Function to check npm authentication
check_npm_auth() {
    if ! command -v npm &> /dev/null; then
        print_warning "npm is not available - skipping npm checks"
        return 1
    fi

    if ! npm whoami &> /dev/null; then
        print_warning "Not logged in to npm"
        print_info "Run: npm login"
        return 1
    fi

    print_success "npm authentication verified"
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

# Function to get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to get current version from vtcode-core/Cargo.toml
get_core_version() {
    grep '^version = ' vtcode-core/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to update version in Cargo.toml files
update_version() {
    local new_version=$1

    # Update main Cargo.toml
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml
    rm Cargo.toml.bak

    # Update vtcode-core Cargo.toml
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" vtcode-core/Cargo.toml
    rm vtcode-core/Cargo.toml.bak

    # Update npm package.json if it exists
    if [[ -f "npm/package.json" ]]; then
        sed -i.bak "s/\"version\": \".*\"/\"version\": \"$new_version\"/" npm/package.json
        rm npm/package.json.bak
    fi

    print_success "Updated version to $new_version in all package files"
}

# Function to validate package metadata
validate_metadata() {
    print_info "Validating package metadata..."

    # Check main Cargo.toml
    if ! grep -q '^description = ' Cargo.toml; then
        print_error "Missing description in Cargo.toml"
        return 1
    fi

    if ! grep -q '^license = ' Cargo.toml; then
        print_error "Missing license in Cargo.toml"
        return 1
    fi

    if ! grep -q '^repository = ' Cargo.toml; then
        print_error "Missing repository in Cargo.toml"
        return 1
    fi

    # Check vtcode-core Cargo.toml
    if ! grep -q '^description = ' vtcode-core/Cargo.toml; then
        print_error "Missing description in vtcode-core/Cargo.toml"
        return 1
    fi

    print_success "Package metadata validation passed"
}

# Function to publish to crates.io
publish_to_crates() {
    local dry_run=$1

    print_distribution "Publishing to crates.io..."

    if [[ "$dry_run" == "true" ]]; then
        print_info "Dry run - checking crates.io publishing"
        if ! cargo publish --dry-run; then
            print_error "Dry run failed for main crate"
            return 1
        fi

        if ! cargo publish --dry-run --manifest-path vtcode-core/Cargo.toml; then
            print_error "Dry run failed for vtcode-core"
            return 1
        fi

        print_success "Crates.io dry run successful"
        return 0
    fi

    # Publish vtcode-core first
    print_info "Publishing vtcode-core to crates.io..."
    if ! cargo publish --manifest-path vtcode-core/Cargo.toml; then
        print_error "Failed to publish vtcode-core"
        return 1
    fi

    print_success "Published vtcode-core to crates.io"

    # Wait for vtcode-core to be available
    print_info "Waiting for vtcode-core to be available on crates.io..."
    sleep 30

    # Publish main crate
    print_info "Publishing vtcode to crates.io..."
    if ! cargo publish; then
        print_error "Failed to publish vtcode"
        return 1
    fi

    print_success "Published vtcode to crates.io"
}

# Function to publish npm package
publish_to_npm() {
    local dry_run=$1

    if [[ ! -d "npm" ]]; then
        print_warning "npm directory not found - skipping npm publishing"
        return 0
    fi

    print_distribution "Publishing to npm..."

    cd npm

    if [[ "$dry_run" == "true" ]]; then
        print_info "Dry run - checking npm publishing"
        if ! npm publish --dry-run; then
            print_error "npm dry run failed"
            cd ..
            return 1
        fi
        print_success "npm dry run successful"
        cd ..
        return 0
    fi

    print_info "Publishing to npm..."
    if ! npm publish; then
        print_error "Failed to publish to npm"
        cd ..
        return 1
    fi

    print_success "Published to npm"
    cd ..
}

# Function to update Homebrew formula
update_homebrew_formula() {
    local version=$1

    if [[ ! -f "homebrew/vtcode.rb" ]]; then
        print_warning "Homebrew formula not found - skipping Homebrew update"
        return 0
    fi

    print_distribution "Updating Homebrew formula..."

    # This would typically be done manually or via a separate workflow
    # For now, just show the instructions
    print_info "Homebrew formula needs manual update:"
    print_info "1. Update version in homebrew/vtcode.rb to $version"
    print_info "2. Update SHA256 hashes for new binaries"
    print_info "3. Commit and push changes to homebrew tap repository"
    print_info "4. Users can then run: brew install vinhnx/tap/vtcode"
}

# Function to update version in vtcode-core/Cargo.toml
update_core_version() {
    local new_version=$1
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" vtcode-core/Cargo.toml
    rm vtcode-core/Cargo.toml.bak
    print_success "Updated vtcode-core version to $new_version"
}

# Function to create git tag
create_tag() {
    local version=$1
    local tag="v$version"

    if git tag -l | grep -q "^$tag$"; then
        print_error "Tag $tag already exists"
        exit 1
    fi

    git tag -a "$tag" -m "Release $tag"
    print_success "Created tag $tag"
}

# Function to push tag to GitHub
push_tag() {
    local version=$1
    local tag="v$version"

    git push origin "$tag"
    print_success "Pushed tag $tag to GitHub"
}

# Function to show usage
show_usage() {
    cat << EOF
VTCode Release Script with Multi-Provider Distribution

USAGE:
    $0 [OPTIONS] [VERSION]

ARGUMENTS:
    VERSION    Version to release (e.g., 1.0.0, 1.2.3)

OPTIONS:
    -h, --help          Show this help message
    -p, --patch         Create a patch release (increment patch version)
    -m, --minor         Create a minor release (increment minor version)
    -M, --major         Create a major release (increment major version)
    --dry-run           Show what would be done without making changes
    --skip-crates       Skip publishing to crates.io
    --skip-npm          Skip publishing to npm
    --skip-homebrew     Skip Homebrew formula update

EXAMPLES:
    $0 1.0.0                           # Release specific version
    $0 --patch                         # Create patch release
    $0 --minor --skip-npm              # Create minor release, skip npm
    $0 --patch --dry-run               # Show what patch release would do

DISTRIBUTION CHANNELS:
    - crates.io: Rust package registry
    - npm: Node.js package registry
    - Homebrew: macOS package manager
    - GitHub Releases: Pre-built binaries

SETUP REQUIREMENTS:
    1. Cargo: Run 'cargo login' with your crates.io API token
    2. npm: Run 'npm login' if publishing to npm
    3. GitHub: Ensure CRATES_IO_TOKEN secret is set for CI publishing

EOF
}

# Function to increment version
increment_version() {
    local current_version=$1
    local increment_type=$2

    # Split version into parts
    IFS='.' read -ra VERSION_PARTS <<< "$current_version"

    case $increment_type in
        patch)
            VERSION_PARTS[2]=$((VERSION_PARTS[2] + 1))
            ;;
        minor)
            VERSION_PARTS[1]=$((VERSION_PARTS[1] + 1))
            VERSION_PARTS[2]=0
            ;;
        major)
            VERSION_PARTS[0]=$((VERSION_PARTS[0] + 1))
            VERSION_PARTS[1]=0
            VERSION_PARTS[2]=0
            ;;
        *)
            print_error "Invalid increment type: $increment_type"
            exit 1
            ;;
    esac

    echo "${VERSION_PARTS[0]}.${VERSION_PARTS[1]}.${VERSION_PARTS[2]}"
}

# Main function
main() {
    local version=""
    local increment_type=""
    local dry_run=false
    local skip_crates=false
    local skip_npm=false
    local skip_homebrew=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -p|--patch)
                increment_type="patch"
                shift
                ;;
            -m|--minor)
                increment_type="minor"
                shift
                ;;
            -M|--major)
                increment_type="major"
                shift
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            --skip-crates)
                skip_crates=true
                shift
                ;;
            --skip-npm)
                skip_npm=true
                shift
                ;;
            --skip-homebrew)
                skip_homebrew=true
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
    if [ -n "$increment_type" ] && [ -n "$version" ]; then
        print_error "Cannot specify both increment type and version"
        exit 1
    fi

    if [ -z "$increment_type" ] && [ -z "$version" ]; then
        print_error "Must specify either version or increment type"
        show_usage
        exit 1
    fi

    # Get current versions
    local current_version=$(get_current_version)
    print_info "Current version: $current_version"
    local current_core_version=$(get_core_version)
    print_info "Current vtcode-core version: $current_core_version"

    # Determine new version
    if [ -n "$increment_type" ]; then
        version=$(increment_version "$current_version" "$increment_type")
        print_info "New version will be: $version"
    else
        print_info "Releasing version: $version"
    fi

    # Pre-flight checks
    print_info "Running pre-flight checks..."
    check_branch
    check_clean_tree
    validate_metadata

    # Check authentication for enabled providers
    if [[ "$skip_crates" != "true" ]]; then
        check_cargo_auth
    fi

    if [[ "$skip_npm" != "true" ]]; then
        check_npm_auth
    fi

    if [[ "$skip_homebrew" != "true" ]]; then
        check_homebrew_setup
    fi

    if [ "$dry_run" = true ]; then
        print_warning "DRY RUN - No changes will be made"
        echo
        echo "Would perform the following actions:"
        echo "1. Update version to $version in all package files"
        echo "2. Create git tag v$version"
        echo "3. Push tag v$version to GitHub"
        if [[ "$skip_crates" != "true" ]]; then
            echo "4. Publish to crates.io (dry run)"
        fi
        if [[ "$skip_npm" != "true" ]]; then
            echo "5. Publish to npm (dry run)"
        fi
        if [[ "$skip_homebrew" != "true" ]]; then
            echo "6. Update Homebrew formula"
        fi
        echo "7. GitHub Actions will create release with binaries"
        exit 0
    fi

    # Handle core version update
    local core_version=""
    if [ "$dry_run" = true ]; then
        # In dry-run mode, use the same version as main package
        core_version="$version"
        print_info "vtcode-core will be bumped to $core_version (dry-run)"
    else
        # Interactive mode - prompt for core version
        echo
        read -p "Enter new vtcode-core version (leave blank to skip): " core_version
        if [ -n "$core_version" ]; then
            print_info "vtcode-core will be bumped to $core_version"
        else
            print_warning "Skipping vtcode-core version bump"
        fi
    fi

    # Confirm release
    echo
    print_warning "This will create a release for version $version"
    echo "Distribution channels:"
    if [[ "$skip_crates" != "true" ]]; then echo "  - crates.io"; fi
    if [[ "$skip_npm" != "true" ]]; then echo "  - npm"; fi
    if [[ "$skip_homebrew" != "true" ]]; then echo "  - Homebrew"; fi
    echo "  - GitHub Releases (binaries)"
    echo
    read -p "Are you sure you want to continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Release cancelled"
        exit 0
    fi

    # Perform release steps
    print_info "Starting release process..."

    # Update version in all package files
    update_version "$version"
    local files_to_commit="Cargo.toml"

    if [ -n "$core_version" ]; then
        update_core_version "$core_version"
        files_to_commit="$files_to_commit vtcode-core/Cargo.toml"
    fi

    # Publish to different providers
    if [[ "$skip_crates" != "true" ]]; then
        if ! publish_to_crates false; then
            print_error "Failed to publish to crates.io"
            exit 1
        fi
    fi

    if [[ "$skip_npm" != "true" ]]; then
        if ! publish_to_npm false; then
            print_error "Failed to publish to npm"
            exit 1
        fi
    fi

    if [[ "$skip_homebrew" != "true" ]]; then
        update_homebrew_formula "$version"
    fi

    # Commit version change
    git add Cargo.toml vtcode-core/Cargo.toml
    if [[ -f "npm/package.json" ]]; then
        git add npm/package.json
    fi
    git commit -m "chore: bump version to $version"
    print_success "Committed version bump"

    # Push commit to GitHub
    git push origin main
    print_success "Pushed commit to GitHub"

    # Create and push tag
    create_tag "$version"
    push_tag "$version"

    print_success "Release $version created successfully!"
    print_info "Distribution Summary:"
    if [[ "$skip_crates" != "true" ]]; then
        print_info "  - Published to crates.io: https://crates.io/crates/vtcode"
    fi
    if [[ "$skip_npm" != "true" ]]; then
        print_info "  - Published to npm: https://www.npmjs.com/package/vtcode"
    fi
    if [[ "$skip_homebrew" != "true" ]]; then
        print_info "  - Homebrew formula updated (manual step required)"
    fi
    print_info "  - GitHub Release: https://github.com/vinhnx/vtcode/releases/tag/v$version"
    print_info "  - Check https://github.com/vinhnx/vtcode/actions for CI status"
}

# Run main function
main "$@"
