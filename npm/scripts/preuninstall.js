#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

function uninstall() {
  try {
    const binDir = path.join(__dirname, '..', 'bin');

    if (fs.existsSync(binDir)) {
      fs.rmSync(binDir, { recursive: true, force: true });
      console.log('VTCode binaries cleaned up successfully!');
    }
  } catch (error) {
    console.error('Failed to clean up VTCode binaries:', error.message);
  }
}

uninstall();