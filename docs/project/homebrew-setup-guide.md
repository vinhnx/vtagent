# VTCode Homebrew Distribution Setup Guide

This guide will help you set up Homebrew distribution for VTCode on macOS.

## Prerequisites

1. **GitHub Repository**: You need a GitHub repository for your Homebrew tap
2. **GitHub Release**: You need to create a GitHub release with pre-built binaries
3. **SHA256 Hashes**: You need to calculate SHA256 hashes for your binaries

## Step 1: Create Homebrew Tap Repository

### Option A: Create a new tap repository

1. Create a new GitHub repository named `homebrew-tap` (or similar)
2. Clone it locally:

```bash
git clone https://github.com/YOUR_USERNAME/homebrew-tap.git
cd homebrew-tap
```

### Option B: Use existing tap repository

If you already have a tap repository, use that instead.

## Step 2: Create the Formula

Create a file named `vtcode.rb` in your tap repository:

```ruby
class Vtcode < Formula
  desc "A Rust-based terminal coding agent with modular architecture"
  homepage "https://github.com/vinhnx/vtcode"
  version "0.8.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinhnx/vtcode/releases/download/v#{version}/vtcode-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "CALCULATE_THIS_SHA256_HASH"
    else
      url "https://github.com/vinhnx/vtcode/releases/download/v#{version}/vtcode-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "CALCULATE_THIS_SHA256_HASH"
    end
  end

  def install
    bin.install "vtcode"
  end

  test do
    system "#{bin}/vtcode", "--version"
  end
end
```

## Step 3: Calculate SHA256 Hashes

After creating a GitHub release with your binaries, calculate the SHA256 hashes:

### For Intel Mac:

```bash
# Download the Intel binary
curl -L -o vtcode-intel.tar.gz https://github.com/vinhnx/vtcode/releases/download/v0.8.1/vtcode-v0.8.1-x86_64-apple-darwin.tar.gz

# Calculate SHA256
shasum -a 256 vtcode-intel.tar.gz
```

### For Apple Silicon Mac:

```bash
# Download the ARM binary
curl -L -o vtcode-arm.tar.gz https://github.com/vinhnx/vtcode/releases/download/v0.8.1/vtcode-v0.8.1-aarch64-apple-darwin.tar.gz

# Calculate SHA256
shasum -a 256 vtcode-arm.tar.gz
```

## Step 4: Update the Formula

Replace `CALCULATE_THIS_SHA256_HASH` with the actual SHA256 hashes in your `vtcode.rb` file.

## Step 5: Commit and Push

```bash
git add vtcode.rb
git commit -m "Add vtcode formula v0.8.1"
git push origin main
```

## Step 6: Test the Formula

Test your formula locally:

```bash
# Test the formula
brew install --build-from-source vtcode.rb

# Verify installation
vtcode --version

# Uninstall for testing
brew uninstall vtcode
```

## Step 7: Update Release Script

Update your release script to automatically update the Homebrew formula. Here's how:

### Option A: Manual Update (Current)

The current release script shows instructions for manual update.

### Option B: Automated Update

To automate Homebrew formula updates, you can:

1. **Use GitHub Actions** to automatically update the formula when a new release is created
2. **Use a script** to update the formula as part of your release process

Example automated update script:

```bash
#!/bin/bash

# Update Homebrew formula
update_homebrew_formula() {
    local version=$1
    local tap_repo="YOUR_USERNAME/homebrew-tap"
    local formula_path="vtcode.rb"

    # Clone or update tap repository
    if [ ! -d "homebrew-tap" ]; then
        git clone "https://github.com/$tap_repo.git" homebrew-tap
    fi

    cd homebrew-tap

    # Update formula with new version and SHA256
    # This would require calculating SHA256 hashes automatically
    # Implementation depends on your CI/CD setup

    cd ..
}
```

## Usage

Once your tap is set up, users can install VTCode with:

```bash
# Add your tap
brew tap YOUR_USERNAME/homebrew-tap

# Install VTCode
brew install YOUR_USERNAME/homebrew-tap/vtcode
```

Or if you name your tap `homebrew-tap`:

```bash
brew tap YOUR_USERNAME/homebrew-tap
brew install vtcode
```

## Maintenance

### Updating the Formula

When you release a new version:

1. Create a new GitHub release with binaries
2. Calculate new SHA256 hashes
3. Update the `vtcode.rb` file in your tap repository
4. Commit and push the changes

### Version Management

Keep your formula version in sync with your main project version. The formula should always point to the latest stable release.

## Troubleshooting

### Common Issues

1. **SHA256 mismatch**: Make sure you're using the correct SHA256 hash for the binary
2. **Architecture issues**: Ensure you have binaries for both Intel and Apple Silicon Macs
3. **Permission issues**: Make sure your tap repository is public or users have access

### Testing

Always test your formula before publishing:

```bash
# Test installation
brew install --build-from-source vtcode.rb

# Test functionality
vtcode --version

# Clean up
brew uninstall vtcode
```

## Integration with Release Script

To integrate with your release script, add this function:

```bash
# Function to update Homebrew formula
update_homebrew_formula() {
    local version=$1
    local tap_repo="YOUR_USERNAME/homebrew-tap"

    print_info "Updating Homebrew formula to version $version"

    # Calculate SHA256 hashes for new binaries
    # This would need to be implemented based on your release process

    print_info "Homebrew formula update instructions:"
    print_info "1. Update version in $tap_repo/vtcode.rb to $version"
    print_info "2. Update SHA256 hashes for new binaries"
    print_info "3. Commit and push changes to $tap_repo"
    print_info "4. Users can then run: brew install $tap_repo/vtcode"
}
```

This setup provides a complete Homebrew distribution system for VTCode that integrates with your existing release process.
