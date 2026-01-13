# MkDocs Setup for GitHub Pages

This document explains how to set up and deploy MkDocs documentation to GitHub Pages.

## What's Been Set Up

✅ **MkDocs Configuration** (`mkdocs.yml`)
- Material theme with dark/light mode
- Navigation structure organized
- Search functionality enabled
- Git revision dates plugin

✅ **GitHub Actions Workflow** (`.github/workflows/docs.yml`)
- Automatic deployment on push to main
- Builds documentation site
- Deploys to GitHub Pages

✅ **Documentation Structure**
- Organized in `docs/` directory
- Organized by category (getting-started, user-guide, developer-guide, etc.)

✅ **Dependencies** (`requirements-docs.txt`)
- MkDocs and Material theme
- Required plugins

## First-Time Setup

### 1. Enable GitHub Pages

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Pages**
3. Under **Source**, select **GitHub Actions**
4. Save the settings

### 2. Test Locally (Optional)

```bash
# Install dependencies
pip install -r requirements-docs.txt

# Serve locally
mkdocs serve

# Open http://127.0.0.1:8000 in your browser
```

### 3. Deploy

The documentation will automatically deploy when you:

1. Push changes to the `main` branch
2. The GitHub Actions workflow will:
   - Build the documentation
   - Deploy to GitHub Pages
   - Make it available at `https://rennerdo30.github.io/toss-share/`

## Manual Deployment

If you want to deploy manually:

```bash
# Build the site
mkdocs build

# The site will be in the `site/` directory
# You can then deploy it manually to GitHub Pages
```

## Updating Documentation

1. Edit files in the `docs/` directory
2. Update `mkdocs.yml` if you need to change navigation
3. Commit and push to `main`
4. GitHub Actions will automatically rebuild and deploy

## Documentation Structure

```
docs/
├── index.md                    # Homepage
├── getting-started/            # Getting started guides
│   ├── quick-start.md
│   ├── installation.md
│   └── development-setup.md
├── user-guide/                 # User documentation
│   ├── overview.md
│   ├── pairing.md
│   └── using-toss.md
├── developer-guide/            # Developer documentation
│   ├── architecture.md
│   ├── api-reference.md
│   ├── platform-support.md
│   └── testing.md
├── platform-specific/          # Platform guides
│   ├── overview.md
│   ├── macos.md
│   ├── windows.md
│   ├── linux.md
│   ├── ios.md
│   ├── android.md
│   └── ios-android.md
├── contributing/               # Contributing
│   ├── contributing.md
│   ├── code-of-conduct.md
│   └── security.md
└── project-status/            # Project status
    ├── status.md
    ├── implementation-summary.md
    ├── todo.md
    └── changelog.md
```

## Customization

### Theme

Edit `mkdocs.yml` to customize the theme:

```yaml
theme:
  name: material
  palette:
    - scheme: default
      primary: indigo  # Change color
      accent: indigo
```

### Navigation

Edit the `nav:` section in `mkdocs.yml` to reorganize navigation.

### Plugins

Add plugins in `mkdocs.yml`:

```yaml
plugins:
  - search
  - git-revision-date-localized-plugin
  - your-plugin
```

## Troubleshooting

### Build Fails

Check the GitHub Actions logs for errors. Common issues:
- Missing dependencies in `requirements-docs.txt`
- Syntax errors in `mkdocs.yml`
- Missing markdown files referenced in navigation

### Pages Not Updating

1. Check GitHub Actions workflow status
2. Verify GitHub Pages is enabled
3. Wait a few minutes for deployment to complete

### Local Build Issues

```bash
# Clean and rebuild
rm -rf site/
mkdocs build
```

## Resources

- [MkDocs Documentation](https://www.mkdocs.org/)
- [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/)
- [GitHub Pages Documentation](https://docs.github.com/en/pages)
