#!/bin/bash

# VTCode Homebrew Formula Update Script
# This script helps update the Homebrew formula when a new release is created

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

# Function to calculate SHA256 for a URL
calculate_sha256() {
    local url=$1
    local filename=$(basename "$url")

    print_info "Downloading $filename..."
    if ! curl -L -o "$filename" "$url"; then
        print_error "Failed to download $filename"
        return 1
    fi

    print_info "Calculating SHA256 for $filename..."
    local sha256
    sha256=$(shasum -a 256 "$filename" | cut -d' ' -f1)

    # Clean up
    rm "$filename"

    echo "$sha256"
}

# Function to update Homebrew formula
update_formula() {
    local version=$1
    local tap_repo=$2
    local formula_path="vtcode.rb"

    print_info "Updating Homebrew formula to version $version"

    # Check if tap repository exists
    if [ ! -d "$tap_repo" ]; then
        print_info "Cloning tap repository..."
        if ! git clone "https://github.com/$tap_repo.git" "$tap_repo"; then
            print_error "Failed to clone tap repository"
            return 1
        fi
    fi

    cd "$tap_repo"

    # Check if formula exists
    if [ ! -f "$formula_path" ]; then
        print_error "Formula file $formula_path not found in tap repository"
        cd ..
        return 1
    fi

    print_info "Calculating SHA256 hashes for binaries..."

    # Intel Mac binary
    local intel_url="https://github.com/vinhnx/vtcode/releases/download/v$version/vtcode-v$version-x86_64-apple-darwin.tar.gz"
    local intel_sha256
    if intel_sha256=$(calculate_sha256 "$intel_url"); then
        print_success "Intel SHA256: $intel_sha256"
    else
        print_error "Failed to calculate Intel SHA256"
        cd ..
        return 1
    fi

    # ARM Mac binary
    local arm_url="https://github.com/vinhnx/vtcode/releases/download/v$version/vtcode-v$version-aarch64-apple-darwin.tar.gz"
    local arm_sha256
    if arm_sha256=$(calculate_sha256 "$arm_url"); then
        print_success "ARM SHA256: $arm_sha256"
    else
        print_error "Failed to calculate ARM SHA256"
        cd ..
        return 1
    fi

    # Update formula
    print_info "Updating formula with new version and SHA256 hashes..."

    # Update version
    sed -i.bak "s/version \".*\"/version \"$version\"/" "$formula_path"
    rm "${formula_path}.bak"

    # Update Intel SHA256
    sed -i.bak "s/vtcode-v.*-x86_64-apple-darwin.tar.gz\"/vtcode-v$version-x86_64-apple-darwin.tar.gz\"/" "$formula_path"
    sed -i.bak "s/sha256 \".*\" # Calculate: shasum -a 256 vtcode-v.*-x86_64-apple-darwin.tar.gz/sha256 \"$intel_sha256\"/" "$formula_path"
    rm "${formula_path}.bak"

    # Update ARM SHA256
    sed -i.bak "s/vtcode-v.*-aarch64-apple-darwin.tar.gz\"/vtcode-v$version-aarch64-apple-darwin.tar.gz\"/" "$formula_path"
    sed -i.bak "s/sha256 \".*\" # Calculate: shasum -a 256 vtcode-v.*-aarch64-apple-darwin.tar.gz/sha256 \"$arm_sha256\"/" "$formula_path"
    rm "${formula_path}.bak"

    print_success "Formula updated successfully"

    # Show diff
    print_info "Changes made to formula:"
    git diff "$formula_path"

    # Ask for confirmation before committing
    read -p "Commit and push these changes? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git add "$formula_path"
        git commit -m "Update vtcode formula to v$version"
        git push origin main
        print_success "Formula committed and pushed to $tap_repo"
    else
        print_info "Changes not committed. You can commit them manually."
    fi

    cd ..
}

# Main function
main() {
    local version=""
    local tap_repo=""

    # Parse arguments
    if [ $# -lt 2 ]; then
        echo "Usage: $0 <version> <tap-repo>"
        echo "Example: $0 0.8.1 vinhnx/homebrew-tap"
        exit 1
    fi

    version=$1
    tap_repo=$2

    print_info "Updating Homebrew formula for VTCode v$version"
    print_info "Tap repository: $tap_repo"

    update_formula "$version" "$tap_repo"
}

# Run main function
main "$@"