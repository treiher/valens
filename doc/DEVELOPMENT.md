# Development

This document explains how to set up, develop, and release Valens. For a high-level description of the architecture and components, see the [Architecture](doc/ARCHITECTURE.md) document.

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

Install the Python project and development dependencies inside the shell.

```console
$ uv sync
```

### 2. Installing the dependencies manually

Install the following tools (with your system package manager):

- [Rust](https://rust-lang.org/tools/install/) (providing the `rustup` command)
- [Python](https://www.python.org/downloads/) and [uv](https://github.com/astral-sh/uv)
- [Dart Sass](https://sass-lang.com/dart-sass) (providing the `sass` command)
- [Chromium](https://www.chromium.org/Home/) and [ChromeDriver](https://sites.google.com/chromium.org/driver/) for browser-based tests

Install the Rust toolchain.

```console
$ rustup show
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

## Running development servers

The current codebase can be executed by running development servers for the frontend and the backend. The development servers will automatically reload when the codebase is changed.

Start both development servers at the same time (requires an active tmux session):

```console
$ make run
```

Alternatively, start the development servers for the frontend and the backend separately:

```console
$ make run_frontend
```

```console
$ make run_backend
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

- [ ] Update screenshots if necessary
- [ ] Update revision used for PyPI README in `pyproject.toml` if necessary
- [ ] Add release to `CHANGELOG`
- [ ] Merge changes into `main` branch
- [ ] Add tag
    - Note: Commit IDs change when a PR is merged on GitHub, so it must be ensured that the `main` branch is checked out.
    - `git tag -a vX.Y.Z -m ""`
- [ ] Push tag
    - `git push --follow-tags`
- [ ] Approve publishing to PyPI
- [ ] Check project on PyPI
- [ ] Test installation from PyPI
    - `pip3 install valens`
- [ ] Publish release notes on GitHub
    - Draft new release
    - Select tag
    - Add corresponding part of `CHANGELOG` as description
    - Publish release
