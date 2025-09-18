#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

function getBinaryPath() {
  const binDir = path.join(__dirname, 'bin');
  const binaryName = process.platform === 'win32' ? 'vtcode.exe' : 'vtcode';
  return path.join(binDir, binaryName);
}

function main() {
  const binaryPath = getBinaryPath();

  if (!fs.existsSync(binaryPath)) {
    console.error('VTCode binary not found. Please reinstall the package.');
    process.exit(1);
  }

  // Spawn the VTCode binary with all arguments passed to this script
  const child = spawn(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
    cwd: process.cwd()
  });

  child.on('exit', (code) => {
    process.exit(code);
  });

  child.on('error', (error) => {
    console.error('Failed to start VTCode:', error.message);
    process.exit(1);
  });
}

main();