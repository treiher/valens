import re
from pathlib import Path

from setuptools import find_packages, setup  # type: ignore

match = re.search(r'__version__ = "(.*?)"', Path("valens/__init__.py").read_text(encoding="utf-8"))
assert match
version = match.group(1)

readme = Path("README.md").read_text(encoding="utf-8")

setup(
    name="valens",
    version=version,
    description="An app for tracking your health and training progress.",
    long_description=readme,
    long_description_content_type="text/markdown",
    author="Tobias Reiher",
    author_email="valens@ardeidae.de",
    url="https://github.com/treiher/valens",
    license="AGPL-3.0",
    classifiers=[
        "Development Status :: 4 - Beta",
        "Environment :: Web Environment",
        "Intended Audience :: End Users/Desktop",
        "License :: OSI Approved :: GNU Affero General Public License v3",
        "Natural Language :: English",
        "Operating System :: POSIX :: Linux",
        "Programming Language :: Python :: 3 :: Only",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Topic :: Other/Nonlisted Topic",
    ],
    packages=find_packages(include=["valens"]),
    package_data={"valens": ["*", "*/*", "*/*/*"]},
    python_requires=">=3.8",
    install_requires=[
        "alembic >=1.6",
        "flask >=2.0.2",
        "matplotlib",
        "pandas",
        "sqlalchemy-repr >= 0.0.2",
        "sqlalchemy[mypy] >= 1.4",
    ],
    extras_require={
        "devel": [
            "black >=21.9b0",
            "flake8 >=3",
            "isort >=5",
            "mypy >=0.910",
            "pylint >=2.11.0",
            "pytest >=5",
            "pytest-alembic >=0.3.1",
            "pytest-cov >=2.10.0",
            "pytest-xdist >=1.32.0",
        ]
    },
    entry_points={"console_scripts": ["valens=valens.cli:main"]},
)
