# Python Wheel Release Guide

This project uses [maturin](https://github.com/PyO3/maturin) to build Python wheels for multiple Python versions and platforms.

## Supported Versions

The build process creates wheels for:
- **Python versions**: 3.8, 3.9, 3.10, 3.11, 3.12, 3.13
- **Platforms**:
  - Linux (x86_64, aarch64)
  - macOS (x86_64, aarch64/Apple Silicon)
  - Windows (x64)

## Local Development

### Install maturin

```bash
pip install maturin
```

### Build a wheel for your current Python version

```bash
maturin build --release --features python
```

The wheel will be created in `target/wheels/`.

### Build and install for development

```bash
maturin develop --features python
```

This builds and installs the package in your current Python environment.

### Test the installation

```bash
python -c "import feedtui; feedtui._cli_main()"
# or
feedtui --help
```

## Building for Multiple Python Versions Locally

If you have multiple Python versions installed (e.g., via pyenv):

```bash
# Build for all Python interpreters found on your system
maturin build --release --features python --find-interpreter

# Or specify specific Python versions
maturin build --release --features python -i python3.8 -i python3.9 -i python3.10
```

## GitHub Actions Workflows

### Automatic PyPI Release (pypi-release.yml)

Triggered when you push a tag:

```bash
git tag v0.1.1
git push origin v0.1.1
```

This workflow will:
1. Build wheels for all supported Python versions and platforms
2. Build a source distribution (sdist)
3. Publish to PyPI (requires `PYPI_API_TOKEN` secret)
4. Create a GitHub release with all wheel artifacts

**Note**: This workflow runs alongside the Rust binary release workflow (`release.yml`), so a single tag will trigger both Rust (crates.io) and Python (PyPI) releases.

### Testing Builds (python-wheels.yml)

Triggered on:
- Pushes to main branch
- Manual dispatch (Actions tab in GitHub)

This workflow builds wheels for all Python versions to ensure compatibility without publishing.

## PyPI Setup

To enable automatic publishing to PyPI:

1. Create a PyPI API token at https://pypi.org/manage/account/token/
2. Add it as a repository secret named `PYPI_API_TOKEN`
   - Go to: Settings → Secrets and variables → Actions → New repository secret

## Manual Publishing

If you prefer to publish manually:

```bash
# Build all wheels
maturin build --release --features python --find-interpreter

# Publish to PyPI
maturin publish --features python
# or
pip install twine
twine upload target/wheels/*
```

## Troubleshooting

### Wheel not compatible with Python version

Make sure you're building with the `--find-interpreter` flag or specifying the correct Python interpreters with `-i`.

### Missing dependencies on Linux

The release workflow uses manylinux for Linux compatibility. If you need additional system dependencies, add them to the `before-script-linux` section in `.github/workflows/release.yml`.

### Testing wheels before release

```bash
# Build the wheel
maturin build --release --features python

# Install it in a fresh virtualenv
python -m venv test_env
source test_env/bin/activate  # or test_env\Scripts\activate on Windows
pip install target/wheels/feedtui-*.whl

# Test it
feedtui --help
```
