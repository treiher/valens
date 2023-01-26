# Valens

[/ˈva.lens/ [ˈväːlɛns] *lat.* strong, vigorous, healthy](https://en.wiktionary.org/wiki/valens#Latin)

![App screenshots](https://raw.githubusercontent.com/treiher/valens/main/doc/screenshots.png "App screenshots")

## Features

- Track your training progress
    - Define training routines
    - Log repetitions, weight, time and rating of perceived exertion (RPE) for each set
    - Measure your training execution using a stopwatch, timer or metronome
    - Assess the progress for each routine and exercise
- Keep track of your body weight
- Calculate and log your body fat based on the 3-site or 7-site caliper method
- Monitor your menstrual cycle (if you have one 😉)

## Installation

```
pip install valens
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

The development server is not intended for production use. Please consider the [deployment options](https://flask.palletsprojects.com/en/2.0.x/deploying/) for providing the app in a public network.

#### Example Configuration: nginx and uWSGI

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

        location = /valens { rewrite ^ /valens/; }
        location /valens { try_files $uri @valens; }
        location @valens {
                include uwsgi_params;
                uwsgi_pass unix:/run/uwsgi/valens.sock;
        }

    }

}
```

## Development

The following software is required:

- Python 3
- Rust toolchain
- tmux (optional)

### Setting up the development environment

Add the WebAssembly target to the Rust toolchain.

```console
$ rustup target add wasm32-unknown-unknown
```

Install the Rust development tools.

```console
$ cargo install --locked trunk
```

Create a Python virtual environment.

```console
$ python3 -m venv .venv
```

Activate the virtual environment.

```console
$ . .venv/bin/activate
```

Install the Python development tools and install the package in editable mode.

```console
$ pip install -e ".[devel]"
```

Create a config file for the backend.

```console
$ make config
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

## License

This project is licensed under the terms of the [AGPL-3.0](https://github.com/treiher/valens/blob/main/LICENSE) license and includes [third-party software](https://github.com/treiher/valens/blob/main/THIRD-PARTY-LICENSES).
