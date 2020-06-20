VERBOSE ?= @
export MYPYPATH = $(PWD)/stubs

python-packages := valens setup.py

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
	python3 -m pytest -vv

test_optimized:
	python3 -O -m pytest -vv

test_coverage:
	coverage run --branch --source=rflx -m pytest -vv
