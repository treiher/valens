SHELL = /bin/bash
VERBOSE ?= @

export SQLALCHEMY_WARN_20=1

python-packages := valens tests
frontend-files := $(addprefix valens/frontend/,index.css index.js index.wasm service-worker.js)

.PHONY: all check check_black check_isort check_pylint check_mypy format \
	test test_installation css dist

all: check test test_installation

check: check_frontend check_backend

check_frontend:
	cargo fmt --manifest-path=frontend/Cargo.toml -- --check
	cargo check --manifest-path=frontend/Cargo.toml
	cargo clippy --manifest-path=frontend/Cargo.toml

check_backend: check_black check_isort check_pylint check_mypy

check_black:
	black --check --diff --line-length 100 $(python-packages)

check_isort:
	isort --check --diff $(python-packages)

check_pylint:
	pylint $(python-packages)

check_mypy:
	mypy --pretty $(python-packages)

format:
	cargo fmt --manifest-path=frontend/Cargo.toml
	black -l 100 $(python-packages)
	isort $(python-packages)

test: test_frontend test_backend

test_frontend:
	cargo test --manifest-path=frontend/Cargo.toml

test_backend:
	mkdir -p valens/frontend
	touch $(frontend-files)
	python3 -m pytest -n$(shell nproc) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing --test-alembic tests

test_installation: dist
	$(eval TMPDIR := $(shell mktemp -d))
	python3 -m venv $(TMPDIR)/venv
	$(TMPDIR)/venv/bin/pip install dist/valens-`python3 -c 'import setuptools_scm; print(setuptools_scm.get_version())'`-py3-none-any.whl
	$(TMPDIR)/venv/bin/valens --help
	rm -rf $(TMPDIR)

css: sass/bulma/bulma.sass
	sass --sourcemap=none sass/bulma.scss:valens/static/css/bulma.css

sass/bulma/bulma.sass:
	wget -qO- https://github.com/jgthms/bulma/releases/download/0.9.3/bulma-0.9.3.zip | bsdtar -xf- -C sass

fonts: sass/fontawesome/scss/fontawesome.scss
	cp sass/fontawesome/webfonts/* frontend/assets/fonts/


sass/fontawesome/scss/fontawesome.scss:
	wget -qO- https://use.fontawesome.com/releases/v5.15.4/fontawesome-free-5.15.4-web.zip | bsdtar -xf- -C sass
	mv sass/fontawesome-* sass/fontawesome

$(frontend-files): sass/bulma/bulma.sass sass/fontawesome/scss/fontawesome.scss $(shell find frontend/src/ -type f -name '*.rs')
	cd frontend && trunk build --release && cd ..
	mkdir -p valens/frontend
	rm -rf valens/frontend/*
	cp frontend/dist/index-*.js valens/frontend/index.js
	cp frontend/dist/index-*_bg.wasm valens/frontend/index.wasm
	cp frontend/dist/index-*.css valens/frontend/index.css
	cp -r frontend/dist/{index.css,service-worker.js,fonts,images,js} valens/frontend

dist: $(frontend-files)
	rm -rf valens.egg-info
	python3 -m build

run:
	FLASK_ENV=development FLASK_APP=valens.web VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0

run_frontend:
	PATH=~/.cargo/bin:${PATH} trunk --config frontend/Trunk.toml serve --port 8000

run_backend:
	FLASK_ENV=development FLASK_APP=valens.api VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0

clean:
	rm -rf valens/frontend
	cd frontend && trunk clean
