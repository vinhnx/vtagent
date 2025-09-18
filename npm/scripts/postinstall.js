#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');

const VERSION = '0.8.0';
const GITHUB_REPO = 'vinhnx/vtcode';

function getPlatformInfo() {
  const platform = os.platform();
  const arch = os.arch();

  let platformName;
  let binaryName;

  switch (platform) {
    case 'darwin':
      platformName = arch === 'arm64' ? 'macos-arm64' : 'macos-x64';
      binaryName = 'vtcode';
      break;
    case 'linux':
      platformName = 'linux-x64';
      binaryName = 'vtcode';
      break;
    case 'win32':
      platformName = 'windows-x64';
      binaryName = 'vtcode.exe';
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  return { platformName, binaryName };
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

async function install() {
  try {
    const { platformName, binaryName } = getPlatformInfo();
    const binDir = path.join(__dirname, '..', 'bin');
    const binaryPath = path.join(binDir, binaryName);

    // Create bin directory if it doesn't exist
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Determine download URL
    const isWindows = os.platform() === 'win32';
    const extension = isWindows ? 'zip' : 'tar.gz';
    const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}/vtcode-v${VERSION}-${platformName}.${extension}`;

    console.log(`Downloading VTCode v${VERSION} for ${platformName}...`);

    // Download the archive
    const archivePath = path.join(binDir, `vtcode.${extension}`);
    await downloadFile(downloadUrl, archivePath);

    // Extract the archive
    if (isWindows) {
      // Use PowerShell to extract zip on Windows
      execSync(`powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '${binDir}' -Force"`);
    } else {
      // Use tar to extract on Unix-like systems
      execSync(`tar -xzf "${archivePath}" -C "${binDir}"`);
    }

    // Clean up archive
    fs.unlinkSync(archivePath);

    // Make binary executable on Unix-like systems
    if (!isWindows) {
      fs.chmodSync(binaryPath, '755');
    }

    console.log('VTCode installed successfully!');
    console.log(`Binary location: ${binaryPath}`);

  } catch (error) {
    console.error('Failed to install VTCode:', error.message);
    process.exit(1);
  }
}

install();