VERBOSE ?= @
export MYPYPATH = $(PWD)/stubs

python-packages := valens tests

.PHONY: all check check_black check_isort check_flake8 check_pylint check_mypy format \
	test test_optimized test_coverage

all: check test

check: check_black check_isort check_flake8 check_pylint check_mypy

check_black:
	black --check --diff --line-length 100 $(python-packages)

check_isort:
	isort --check --diff $(python-packages)

check_flake8:
	flake8 $(python-packages)

check_pylint:
	pylint $(python-packages)

check_mypy:
	mypy --pretty $(python-packages)

format:
	black -l 100 $(python-packages)
	isort $(python-packages)

test:
	pytest -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing tests

css: sass/bulma/bulma.sass
	sass --sourcemap=none sass/bulma.scss:valens/static/css/bulma.css

sass/bulma/bulma.sass:
	wget -qO- https://github.com/jgthms/bulma/releases/download/0.9.2/bulma-0.9.2.zip | bsdtar -xf- -C sass
