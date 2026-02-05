#!/usr/bin/env node

/**
 * Generate PNG icons from SVG source
 *
 * Requires sharp: npm install sharp
 *
 * Usage: node generate-icons.js
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const SVG_PATH = path.join(ROOT, 'src', 'shared', 'icons', 'icon.svg');
const OUTPUT_DIR = path.join(ROOT, 'src', 'shared', 'icons');

const SIZES = [16, 32, 48, 128];

async function generateIcons() {
  // Check if sharp is available
  let sharp;
  try {
    sharp = require('sharp');
  } catch {
    console.error('sharp is not installed. Run: npm install sharp');
    console.log('Creating placeholder icons instead...');
    createPlaceholderIcons();
    return;
  }

  // Check if SVG exists
  if (!fs.existsSync(SVG_PATH)) {
    console.error(`SVG not found: ${SVG_PATH}`);
    process.exit(1);
  }

  console.log('Generating icons from SVG...');

  for (const size of SIZES) {
    const outputPath = path.join(OUTPUT_DIR, `icon${size}.png`);

    await sharp(SVG_PATH)
      .resize(size, size)
      .png()
      .toFile(outputPath);

    console.log(`  Created icon${size}.png`);
  }

  console.log('Icon generation complete!');
}

function createPlaceholderIcons() {
  // Create simple placeholder PNGs
  // This is a minimal valid 1x1 PNG with a purple pixel (#6366f1)
  const png = Buffer.from([
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
    0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
    0x54, 0x08, 0xd7, 0x63, 0x68, 0x60, 0xf8, 0xcf,
    0x80, 0x00, 0x00, 0x03, 0x8c, 0x01, 0x85, 0x9b,
    0xd6, 0x31, 0xf4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
  ]);

  for (const size of SIZES) {
    const outputPath = path.join(OUTPUT_DIR, `icon${size}.png`);
    fs.writeFileSync(outputPath, png);
    console.log(`  Created placeholder icon${size}.png`);
  }
}

generateIcons().catch(console.error);
