class Vtcode < Formula
  desc "A Rust-based terminal coding agent with modular architecture"
  homepage "https://github.com/vinhnx/vtcode"
  version "0.8.2"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinhnx/vtcode/releases/download/v#{version}/vtcode-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "CHANGE_THIS_SHA256_AFTER_FIRST_RELEASE" # Calculate: shasum -a 256 vtcode-v0.8.2-aarch64-apple-darwin.tar.gz
    else
      url "https://github.com/vinhnx/vtcode/releases/download/v#{version}/vtcode-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "CHANGE_THIS_SHA256_AFTER_FIRST_RELEASE" # Calculate: shasum -a 256 vtcode-v0.8.2-x86_64-apple-darwin.tar.gz
    end
  end

  def install
    bin.install "vtcode"
  end

  test do
    system "#{bin}/vtcode", "--version"
  end
end