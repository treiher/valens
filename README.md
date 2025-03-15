# Valens

[/ˈva.lens/ [ˈväːlɛns] *lat.* strong, vigorous, healthy](https://en.wiktionary.org/wiki/valens#Latin)

![App screenshots](https://raw.githubusercontent.com/treiher/valens/main/doc/screenshots.png "App screenshots")

## Features

- Track your training progress
    - Define training routines
    - Choose from more than 150 exercises or create your own
    - Log repetitions, weight, time and rating of perceived exertion (RPE) for each set
    - Measure your training execution using a stopwatch, timer or metronome
    - Assess the progress for each routine and exercise
- Keep track of your body weight
- Calculate and log your body fat based on the 3-site or 7-site caliper method
- Monitor your menstrual cycle (if you have one 😉)

## Installation

The latest release can be installed from [PyPI](https://pypi.org/p/valens).

```
pip install valens
```

The latest development version can be installed from [TestPyPI](https://test.pypi.org/p/valens).

```
pip install --pre --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/ valens
```

## Demo Mode

To get a first impression of Valens, the app can be run in demo mode.

```
valens demo
```

The app can be accessed on `http://127.0.0.1:5000/`. A temporary database with random example data is used. All changes are non-persistent. Adding `--public` to the command line makes the app available to other devices on your network.

## Configuration and Running

A configuration file must be created before running the app for the first time.

```
valens config
```

The environment variable `VALENS_CONFIG` must be set to the *absolute* path of the created config file.

### Local Network

The development server can be used to provide the app for your local computer or local network.

```
VALENS_CONFIG=$PWD/config.py valens run
```

By default, the app is only accessible on your local computer at `http://127.0.0.1:5000/`. If you trust the users in your network, you can make the server publicly available adding `--public` to the command line:

```
VALENS_CONFIG=$PWD/config.py valens run --public
```

### Public Network

The development server is not intended for production use. Please consider the [deployment options](https://flask.palletsprojects.com/en/2.3.x/deploying/) for providing the app in a public network.

#### Example Configuration: NGINX and uWSGI

The following configuration binds the app to `/valens`.

`/etc/uwsgi/valens.ini`

```ini
[uwsgi]
master = true
plugins = python
socket = /run/uwsgi/%n.sock
manage-script-name = true
mount = /valens=valens.web:app
uid = http
gid = http
env = VALENS_CONFIG=/opt/valens/config.py
```

`/etc/nginx/nginx.conf`

```nginx
[...]

http {

    [...]

    server {

        [...]

        gzip on;
        gzip_types text/plain test/css text/javascript application/json application/wasm;

        location = /valens { return 301 /valens/; }
        location /valens/ { try_files $uri @valens; }
        location @valens {
                include uwsgi_params;
                uwsgi_pass unix:/run/uwsgi/valens.sock;
        }

    }

}
```

NGINX compression is disabled by default.
With compression enabled, the amount of data transferred can be significantly reduced, resulting in a reduction in transfer time, especially on slow networks.

## Development

The following software is required:

- Python 3
- [uv](https://github.com/astral-sh/uv)
- Rust toolchain
- tmux (optional)

### Setting up the development environment

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

### Running development servers

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

### Building a distribution package

```console
$ make dist
```

### Deploying the application

Deploy the latest distribution package.

```console
$ fab -H user@host deploy
```

### Changing the database schema

Create a migration script after changing the SQLAlchemy ORM model.

```console
$ VALENS_CONFIG=$PWD/build/config.py alembic revision --autogenerate -m "Add foo table"
```

The automatically generated migration script may be incomplete.

Upgrade the database schema to the latest revision.

```console
$ VALENS_CONFIG=$PWD/build/config.py alembic upgrade head
```

## License

This project is licensed under the terms of the [AGPL-3.0](https://github.com/treiher/valens/blob/main/LICENSE) license and includes [third-party software](https://github.com/treiher/valens/blob/main/THIRD-PARTY-LICENSES).
