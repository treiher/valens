VERBOSE ?= @
export MYPYPATH = $(PWD)/stubs

python-packages := valens tests setup.py

.PHONY: all check check_black check_isort check_flake8 check_pylint check_mypy format \
	test test_optimized test_coverage

all: check test

check: check_black check_isort check_flake8 check_pylint check_mypy

check_black:
	black -l 100 --check $(python-packages)

check_isort:
	isort -rc -c $(python-packages)

check_flake8:
	flake8 $(python-packages)

check_pylint:
	pylint $(python-packages)

check_mypy:
	mypy $(python-packages)

format:
	black -l 100 $(python-packages)
	isort -rc $(python-packages)

test:
	pytest -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing tests
