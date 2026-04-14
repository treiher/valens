# Valens

[/ˈva.lens/ [ˈväːlɛns] *lat.* strong, vigorous, healthy](https://en.wiktionary.org/wiki/valens#Latin)

![App screenshots](doc/screenshots.png "App screenshots")

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

### PyPI

The latest release can be installed from [PyPI](https://pypi.org/p/valens).

```
pip install valens
```

The latest development version can be installed from [TestPyPI](https://test.pypi.org/p/valens).

```
pip install --pre --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/ valens
```

### Container Image

The latest release is available as a container image on the [GitHub Container Registry](https://ghcr.io/treiher/valens).

```
ghcr.io/treiher/valens:latest
```

The latest development version is available under the `dev` tag.

```
ghcr.io/treiher/valens:dev
```

## Running

### PyPI

#### Demo Mode

To get a first impression of Valens, the app can be run in demo mode.

```
valens demo
```

The app can be accessed on `http://127.0.0.1:5000/`. A temporary database with random example data is used. All changes are non-persistent. Adding `--public` to the command line makes the app available to other devices on your network.

#### Local Network

A configuration file must be created before running the app for the first time.

```
valens config
```

The environment variable `VALENS_CONFIG` must be set to the *absolute* path of the created config file.

The development server can be used to provide the app for your local computer or local network.

```
VALENS_CONFIG=$PWD/config.py valens run
```

By default, the app is only accessible on your local computer at `http://127.0.0.1:5000/`. If you trust the users in your network, you can make the server publicly available adding `--public` to the command line:

```
VALENS_CONFIG=$PWD/config.py valens run --public
```

#### Public Network

The development server is not intended for production use. Please consider the [deployment options](https://flask.palletsprojects.com/en/stable/deploying/) for providing the app in a public network. One option is using NGINX and uWSGI.

`/etc/uwsgi/valens.ini`

```ini
[uwsgi]
master = true
plugins = python
socket = /run/uwsgi/%n.sock
manage-script-name = true
mount = /=valens:app
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
        gzip_types text/plain text/css text/javascript application/json application/wasm;

        location / { try_files $uri @valens; }
        location @valens {
            include uwsgi_params;
            uwsgi_pass unix:/run/uwsgi/valens.sock;
        }

    }

}
```

NGINX compression is disabled by default.
With compression enabled, the amount of data transferred can be significantly reduced, resulting in a reduction in transfer time, especially on slow networks.
[Brotli](https://github.com/google/ngx_brotli) offers better compression ratios than gzip and is supported as an optional NGINX module.

### Container

The container image uses [Gunicorn](https://gunicorn.org/) and listens on port 8000. A volume mounted at `/app` provides persistent storage for the database and configuration.

#### Docker / Podman

```
podman run -d -p 8000:8000 -v valens:/app:Z ghcr.io/treiher/valens:latest
```

The app can be accessed on `http://127.0.0.1:8000/`. Replace `podman` with `docker` and omit the `:Z` SELinux label flag when using Docker.

#### Systemd (Quadlet)

The container can be managed as a systemd service using [Podman Quadlet](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html).

`~/.config/containers/systemd/valens.container`

```ini
[Unit]
Description=Valens

[Container]
Image=ghcr.io/treiher/valens:latest
PublishPort=8000:8000
Volume=%h/valens:/app:Z

[Service]
Restart=on-failure

[Install]
WantedBy=default.target
```

```
systemctl --user daemon-reload
systemctl --user enable --now valens
```

#### Public Network

A reverse proxy such as NGINX can be used to expose the container in a public network.

`/etc/nginx/nginx.conf`

```nginx
[...]

http {

    [...]

    server {

        [...]

        gzip on;
        gzip_types text/plain text/css text/javascript application/json application/wasm;

        location / {
            proxy_pass http://127.0.0.1:8000;
        }

    }

}
```

NGINX compression is disabled by default.
With compression enabled, the amount of data transferred can be significantly reduced, resulting in a reduction in transfer time, especially on slow networks.
[Brotli](https://github.com/google/ngx_brotli) offers better compression ratios than gzip and is supported as an optional NGINX module.

## Documentation

- [Development](doc/DEVELOPMENT.md)
- [Architecture](doc/ARCHITECTURE.md)

## License

This project is licensed under the terms of the [AGPL-3.0](LICENSE) license and includes [third-party software](THIRD-PARTY-LICENSES).
