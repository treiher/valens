VERBOSE ?= @

export SQLALCHEMY_WARN_20=1

python-packages := valens tests

.PHONY: all check check_black check_isort check_pylint check_mypy format \
	test test_installation css dist

all: check test test_installation

check: check_black check_isort check_pylint check_mypy

check_black:
	black --check --diff --line-length 100 $(python-packages)

check_isort:
	isort --check --diff $(python-packages)

check_pylint:
	pylint $(python-packages)

check_mypy:
	mypy --pretty $(python-packages)

format:
	black -l 100 $(python-packages)
	isort $(python-packages)

test:
	python3 -m pytest -n$(shell nproc) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing --test-alembic tests

test_installation: dist
	$(eval TMPDIR := $(shell mktemp -d))
	pip wheel setuptools wheel -w $(TMPDIR)/wheels
	pip install valens --no-deps --no-index --find-links dist/ --find-links $(TMPDIR)/wheels/ --target $(TMPDIR)
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

dist:
	rm -rf valens.egg-info
	python3 -m build

run:
	FLASK_ENV=development FLASK_APP=valens.web VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0

run_frontend:
	PATH=~/.cargo/bin:${PATH} trunk --config frontend/Trunk.toml serve --port 8000

run_backend:
	FLASK_ENV=development FLASK_APP=valens.api VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0
