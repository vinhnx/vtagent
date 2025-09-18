#!/bin/bash

# VTAgent Release Script
# This script helps create releases for VTAgent using changelogithub

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# Function to get current version from Cargo.toml
get_current_version() {
    grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
}

# Function to update version in Cargo.toml
update_version() {
    local new_version=$1
    sed -i.bak "s/^version = \".*\"/version = \"$new_version\"/" Cargo.toml
    rm Cargo.toml.bak
    print_success "Updated version to $new_version in Cargo.toml"
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
VTAgent Release Script

USAGE:
    $0 [OPTIONS] [VERSION]

ARGUMENTS:
    VERSION    Version to release (e.g., 1.0.0, 1.2.3)

OPTIONS:
    -h, --help     Show this help message
    -p, --patch    Create a patch release (increment patch version)
    -m, --minor    Create a minor release (increment minor version)
    -M, --major    Create a major release (increment major version)
    --dry-run      Show what would be done without making changes

EXAMPLES:
    $0 1.0.0                    # Release specific version
    $0 --patch                   # Create patch release (e.g., 0.1.0 -> 0.1.1)
    $0 --minor                   # Create minor release (e.g., 0.1.0 -> 0.2.0)
    $0 --major                   # Create major release (e.g., 0.1.0 -> 1.0.0)
    $0 --patch --dry-run         # Show what patch release would do

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

    # Get current version
    local current_version=$(get_current_version)
    print_info "Current version: $current_version"

    # Determine new version
    if [ -n "$increment_type" ]; then
        version=$(increment_version "$current_version" "$increment_type")
        print_info "New version will be: $version"
    else
        print_info "Releasing version: $version"
    fi

    # Pre-flight checks
    check_branch
    check_clean_tree

    if [ "$dry_run" = true ]; then
        print_warning "DRY RUN - No changes will be made"
        echo
        echo "Would perform the following actions:"
        echo "1. Update Cargo.toml version to $version"
        echo "2. Create git tag v$version"
        echo "3. Push tag v$version to GitHub"
        echo "4. GitHub Actions will create release with changelog"
        exit 0
    fi

    # Confirm release
    echo
    print_warning "This will create a release for version $version"
    read -p "Are you sure you want to continue? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Release cancelled"
        exit 0
    fi

    # Perform release steps
    print_info "Starting release process..."

    # Update version in Cargo.toml
    update_version "$version"

    # Commit version change
    git add Cargo.toml
    git commit -m "chore: bump version to $version"
    print_success "Committed version bump"

    # Push commit to GitHub
    git push origin main
    print_success "Pushed commit to GitHub"

    # Create and push tag
    create_tag "$version"
    push_tag "$version"

    print_success "Release $version created successfully!"
    print_info "GitHub Actions will now create the release with changelog"
    print_info "Check https://github.com/vinhnx/vtagent/releases for the release"
}

# Run main function
main "$@"