# Development

The following software is required:

- Python 3
- [uv](https://github.com/astral-sh/uv)
- Rust toolchain
- tmux (optional)

## Setting up the development environment

Add the WebAssembly target to the Rust toolchain.

```console
$ rustup target add wasm32-unknown-unknown
```

Install the Rust development tools.

```console
$ cargo install --locked trunk cargo-llvm-cov cargo-nextest
```

Install the Python project and development tools.

```console
$ uv sync
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
