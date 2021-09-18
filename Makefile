VERBOSE ?= @

export SQLALCHEMY_WARN_20=1

python-packages := valens tests setup.py

.PHONY: all check check_black check_isort check_pylint check_mypy format \
	test test_installation

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

test_installation:
	python setup.py sdist
	$(eval TMPDIR := $(shell mktemp -d))
	pip wheel setuptools wheel -w $(TMPDIR)/wheels
	pip install valens --no-deps --no-index --find-links dist/ --find-links $(TMPDIR)/wheels/ --target $(TMPDIR)
	rm -rf $(TMPDIR)

css: sass/bulma/bulma.sass
	sass --sourcemap=none sass/bulma.scss:valens/static/css/bulma.css

sass/bulma/bulma.sass:
	wget -qO- https://github.com/jgthms/bulma/releases/download/0.9.2/bulma-0.9.2.zip | bsdtar -xf- -C sass
