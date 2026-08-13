# Development

This document explains how to set up, develop, and release Valens. For a high-level description of the architecture and components, see the [Architecture](ARCHITECTURE.md) document.

## Setting up the development environment

The development environment can be set up in two ways:

- Nix with flakes enabled
- A manual installation of the required tools

### 1. Using Nix

This repository includes a preconfigured `.envrc` for [direnv](https://direnv.net/) and [nix-direnv](https://github.com/nix-community/nix-direnv), which will automatically load and unload the Nix development environment when you enter or leave the project directory. To enable this, install `direnv` and run `direnv allow` in the repository root.

Alternatively, you can enter the development shell manually:

```console
$ nix develop
```

### 2. Installing the dependencies manually

Install the following tools (with your system package manager):

- [Rust](https://rust-lang.org/tools/install/) (providing the `rustup` command)
- [Python](https://www.python.org/downloads/) and [uv](https://github.com/astral-sh/uv)
- [Dart Sass](https://sass-lang.com/dart-sass) (providing the `sass` command)
- [Chromium](https://www.chromium.org/Home/), [ChromeDriver](https://sites.google.com/chromium.org/driver/) and [Playwright](https://playwright.dev/python/) for browser-based tests

Install the Rust toolchain.

```console
$ rustup show
```

Install the nightly toolchain that is used for formatting and make it available to `cargo fmt`. The channel is pinned in `rustfmt-toolchain.toml`. The Nix development environment provides this automatically.

```console
$ channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rustfmt-toolchain.toml)
$ rustup toolchain install "$channel" --profile minimal --component rustfmt
$ export RUSTFMT=$(rustup which --toolchain "$channel" rustfmt)
```

Install the Rust-based command-line tools with Cargo or your system package manager.

```console
$ cargo install --locked cargo-llvm-cov cargo-nextest wasm-pack dioxus-cli
```

Then install the Python project and development dependencies.

```console
$ uv sync
```

Activate the Python virtual environment.

```console
$ source .venv/bin/activate
```

The screenshots in the documentation can only be created and checked in the Nix development environment. It pins the font rendering by setting `FONTCONFIG_FILE` to the configuration in `tools/fonts.conf`, which is required to get identical results on different systems.

## Running development servers

The current codebase can be executed by running development servers for the frontend and the backend. The development servers will automatically reload when the codebase is changed.

Start both development servers at the same time (requires an active tmux session):

```console
$ make run
```

Alternatively, start the development servers for the frontend and the backend separately:

```console
$ make run-frontend
```

```console
$ make run-backend
```

After a successful start of the development servers, the web app can be reached on `http://127.0.0.1:8000`.

## Building a distribution package

```console
$ make dist
```

## Deploying the application

Deploy the latest distribution package.

```console
$ fab -H user@host deploy
```

## Changing the database schema

Create a migration script after changing the SQLAlchemy ORM model.

```console
$ VALENS_CONFIG=$PWD/build/config.py alembic revision --autogenerate -m "Add foo table"
```

The automatically generated migration script may be incomplete.

Upgrade the database schema to the latest revision.

```console
$ VALENS_CONFIG=$PWD/build/config.py alembic upgrade head
```

## Release checklist

- [ ] Create release pull request
    - Note: The release version is derived from the changelog by incrementing the current version. The release is added to `CHANGELOG` and the revision used for the PyPI README in `pyproject.toml` is set to the new tag. If the screenshots do not match the current app, they are updated in a separate commit.
    - Note: `make release RELEASE_VERSION=X.Y.Z` creates the release commit locally, if the workflow is unavailable.
    - `gh workflow run release-pr.yml -f increment=patch|minor|major`
- [ ] Merge pull request into `main` branch
    - Note: The screenshots are checked automatically on pull requests that add a release.
- [ ] Approve publishing to PyPI
    - Note: The tag is added and pushed automatically after the release is merged. `make tag` and `git push --follow-tags` do the same on an up-to-date `main` branch.
