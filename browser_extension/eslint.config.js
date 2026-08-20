'use strict';

const js = require('@eslint/js');
const globals = require('globals');

/** Ignored globally: build output and installed dependencies. */
const IGNORED_PATHS = ['dist/**', 'node_modules/**'];

/** ECMAScript version the extension sources target. */
const ECMA_VERSION = 2022;

module.exports = [
  { ignores: IGNORED_PATHS },
  js.configs.recommended,
  {
    files: ['**/*.js'],
    languageOptions: {
      ecmaVersion: ECMA_VERSION,
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.webextensions,
        chrome: 'readonly',
        browser: 'readonly',
      },
    },
    rules: {
      'no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      'no-console': 'off',
      semi: ['error', 'always'],
      quotes: ['error', 'single', { avoidEscape: true }],
      indent: ['error', 2],
      'comma-dangle': ['error', 'always-multiline'],
      eqeqeq: ['error', 'always'],
      curly: ['error', 'all'],
      'brace-style': ['error', '1tbs'],
      'no-var': 'error',
      'prefer-const': 'error',
    },
  },
  {
    // Build and icon-generation scripts run in Node, not the browser.
    files: ['scripts/**/*.js'],
    languageOptions: {
      sourceType: 'commonjs',
      globals: globals.node,
    },
  },
];
