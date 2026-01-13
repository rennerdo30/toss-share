# MkDocs Documentation

This directory contains the source files for the Toss documentation site, built with [MkDocs](https://www.mkdocs.org/) and [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).

## Local Development

### Prerequisites

- Python 3.11+
- pip

### Setup

1. Install dependencies:
   ```bash
   pip install -r requirements-docs.txt
   ```

2. Serve locally:
   ```bash
   mkdocs serve
   ```

3. Open http://127.0.0.1:8000 in your browser

### Build

Build the documentation site:

```bash
mkdocs build
```

The site will be generated in the `site/` directory.

## Deployment

Documentation is automatically deployed to GitHub Pages via GitHub Actions when changes are pushed to the `main` branch.

See `.github/workflows/docs.yml` for the deployment workflow.

## Documentation Structure

- `index.md` - Homepage
- `getting-started/` - Getting started guides
- `user-guide/` - User documentation
- `developer-guide/` - Developer documentation
- `platform-specific/` - Platform-specific guides
- `contributing/` - Contributing guidelines
- `project-status/` - Project status and TODO
- `future-enhancements.md` - Future features
- `specification.md` - Technical specification

## Configuration

The MkDocs configuration is in `mkdocs.yml` at the project root.
