#!/usr/bin/env node

/**
 * Toss Browser Extension Build Script
 *
 * Builds the extension for Chrome (Manifest V3) and Firefox (Manifest V2).
 * Copies shared resources and browser-specific files to dist directory.
 */

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const SRC = path.join(ROOT, 'src');
const DIST = path.join(ROOT, 'dist');

// Parse command line arguments
const args = process.argv.slice(2);
const targetArg = args.find(a => a.startsWith('--target='));
const target = targetArg ? targetArg.split('=')[1] : 'all';
const watch = args.includes('--watch');

/**
 * Copy a file or directory recursively
 */
function copySync(src, dest) {
  const stat = fs.statSync(src);

  if (stat.isDirectory()) {
    if (!fs.existsSync(dest)) {
      fs.mkdirSync(dest, { recursive: true });
    }
    const entries = fs.readdirSync(src);
    for (const entry of entries) {
      copySync(path.join(src, entry), path.join(dest, entry));
    }
  } else {
    fs.copyFileSync(src, dest);
  }
}

/**
 * Create placeholder PNG icons from SVG
 * Note: For production, use sharp or another image library
 */
function createPlaceholderIcons(targetDir) {
  const sizes = [16, 32, 48, 128];
  const iconsDir = path.join(targetDir, 'icons');

  if (!fs.existsSync(iconsDir)) {
    fs.mkdirSync(iconsDir, { recursive: true });
  }

  // Copy SVG as reference
  const svgSrc = path.join(SRC, 'shared', 'icons', 'icon.svg');
  if (fs.existsSync(svgSrc)) {
    fs.copyFileSync(svgSrc, path.join(iconsDir, 'icon.svg'));
  }

  // Create simple placeholder PNGs (1x1 purple pixel as placeholder)
  // In production, use sharp to convert SVG to PNG at various sizes
  for (const size of sizes) {
    const pngPath = path.join(iconsDir, `icon${size}.png`);

    // Create a minimal valid PNG (1x1 purple pixel)
    // This is a valid 1x1 PNG that will work as a placeholder
    const png = Buffer.from([
      0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
      0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
      0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
      0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
      0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IDAT chunk
      0x54, 0x08, 0xd7, 0x63, 0x68, 0x60, 0xf8, 0xcf,
      0x80, 0x00, 0x00, 0x03, 0x8c, 0x01, 0x85, 0x9b,
      0xd6, 0x31, 0xf4, 0x00, 0x00, 0x00, 0x00, 0x49, // IEND chunk
      0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]);

    fs.writeFileSync(pngPath, png);
  }

  console.log(`  Created placeholder icons in ${iconsDir}`);
}

/**
 * Build Chrome extension
 */
function buildChrome() {
  console.log('Building Chrome extension...');

  const chromeDir = path.join(DIST, 'chrome');

  // Clean and create directory
  if (fs.existsSync(chromeDir)) {
    fs.rmSync(chromeDir, { recursive: true });
  }
  fs.mkdirSync(chromeDir, { recursive: true });

  // Copy manifest
  fs.copyFileSync(
    path.join(SRC, 'chrome', 'manifest.json'),
    path.join(chromeDir, 'manifest.json')
  );

  // Copy background script
  fs.copyFileSync(
    path.join(SRC, 'chrome', 'background.js'),
    path.join(chromeDir, 'background.js')
  );

  // Copy shared JS files
  const sharedJsDir = path.join(chromeDir, 'shared', 'js');
  fs.mkdirSync(sharedJsDir, { recursive: true });
  copySync(path.join(SRC, 'shared', 'js'), sharedJsDir);

  // Copy and rename HTML files to root
  fs.copyFileSync(
    path.join(SRC, 'shared', 'html', 'popup.html'),
    path.join(chromeDir, 'popup.html')
  );
  fs.copyFileSync(
    path.join(SRC, 'shared', 'html', 'options.html'),
    path.join(chromeDir, 'options.html')
  );

  // Fix paths in HTML files for Chrome's flat structure
  let popupHtml = fs.readFileSync(path.join(chromeDir, 'popup.html'), 'utf8');
  popupHtml = popupHtml
    .replace('../css/popup.css', 'shared/css/popup.css')
    .replace('../icons/', 'icons/')
    .replace('../js/popup.js', 'shared/js/popup.js');
  fs.writeFileSync(path.join(chromeDir, 'popup.html'), popupHtml);

  let optionsHtml = fs.readFileSync(path.join(chromeDir, 'options.html'), 'utf8');
  optionsHtml = optionsHtml
    .replace('../css/options.css', 'shared/css/options.css')
    .replace('../icons/', 'icons/')
    .replace('../js/options.js', 'shared/js/options.js');
  fs.writeFileSync(path.join(chromeDir, 'options.html'), optionsHtml);

  // Copy CSS files
  const sharedCssDir = path.join(chromeDir, 'shared', 'css');
  fs.mkdirSync(sharedCssDir, { recursive: true });
  copySync(path.join(SRC, 'shared', 'css'), sharedCssDir);

  // Create icons
  createPlaceholderIcons(chromeDir);

  console.log('  Chrome extension built to dist/chrome/');
}

/**
 * Build Firefox extension
 */
function buildFirefox() {
  console.log('Building Firefox extension...');

  const firefoxDir = path.join(DIST, 'firefox');

  // Clean and create directory
  if (fs.existsSync(firefoxDir)) {
    fs.rmSync(firefoxDir, { recursive: true });
  }
  fs.mkdirSync(firefoxDir, { recursive: true });

  // Copy manifest
  fs.copyFileSync(
    path.join(SRC, 'firefox', 'manifest.json'),
    path.join(firefoxDir, 'manifest.json')
  );

  // Copy background script
  fs.copyFileSync(
    path.join(SRC, 'firefox', 'background.js'),
    path.join(firefoxDir, 'background.js')
  );

  // Copy shared JS files
  const sharedJsDir = path.join(firefoxDir, 'shared', 'js');
  fs.mkdirSync(sharedJsDir, { recursive: true });
  copySync(path.join(SRC, 'shared', 'js'), sharedJsDir);

  // Copy and rename HTML files to root
  fs.copyFileSync(
    path.join(SRC, 'shared', 'html', 'popup.html'),
    path.join(firefoxDir, 'popup.html')
  );
  fs.copyFileSync(
    path.join(SRC, 'shared', 'html', 'options.html'),
    path.join(firefoxDir, 'options.html')
  );

  // Fix paths in HTML files
  let popupHtml = fs.readFileSync(path.join(firefoxDir, 'popup.html'), 'utf8');
  popupHtml = popupHtml
    .replace('../css/popup.css', 'shared/css/popup.css')
    .replace('../icons/', 'icons/')
    .replace('../js/popup.js', 'shared/js/popup.js');
  fs.writeFileSync(path.join(firefoxDir, 'popup.html'), popupHtml);

  let optionsHtml = fs.readFileSync(path.join(firefoxDir, 'options.html'), 'utf8');
  optionsHtml = optionsHtml
    .replace('../css/options.css', 'shared/css/options.css')
    .replace('../icons/', 'icons/')
    .replace('../js/options.js', 'shared/js/options.js');
  fs.writeFileSync(path.join(firefoxDir, 'options.html'), optionsHtml);

  // Copy CSS files
  const sharedCssDir = path.join(firefoxDir, 'shared', 'css');
  fs.mkdirSync(sharedCssDir, { recursive: true });
  copySync(path.join(SRC, 'shared', 'css'), sharedCssDir);

  // Create icons
  createPlaceholderIcons(firefoxDir);

  console.log('  Firefox extension built to dist/firefox/');
}

/**
 * Main build function
 */
function build() {
  console.log('Toss Browser Extension Build');
  console.log('============================\n');

  // Create dist directory
  if (!fs.existsSync(DIST)) {
    fs.mkdirSync(DIST, { recursive: true });
  }

  // Build based on target
  if (target === 'all' || target === 'chrome') {
    buildChrome();
  }

  if (target === 'all' || target === 'firefox') {
    buildFirefox();
  }

  console.log('\nBuild complete!');
}

// Run build
build();

// Watch mode
if (watch) {
  console.log('\nWatching for changes...');

  const chokidar = require('chokidar');

  chokidar.watch(SRC, {
    ignored: /node_modules/,
    persistent: true,
  }).on('change', (filePath) => {
    console.log(`\nFile changed: ${filePath}`);
    build();
  });
}
