import re

from setuptools import find_packages, setup  # type: ignore

with open("valens/__init__.py") as f:
    match = re.search(r'__version__ = "(.*?)"', f.read())
    assert match
    version = match.group(1)

setup(
    name="valens",
    version=version,
    license="AGPL-3.0",
    packages=find_packages(include=["valens"]),
    python_requires=">=3.8",
    install_requires=["flask", "matplotlib", "pandas", "pyarrow"],
    extras_require={
        "devel": [
            "black ==20.8b1",
            "flake8 >=3",
            "isort >=5",
            "mypy >=0.770",
            "pylint >=2.6.0",
            "pytest >=5",
            "pytest-cov >=2.10.0",
            "pytest-xdist >=1.32.0",
        ]
    },
    entry_points={"console_scripts": ["valens=valens.cli:main"]},
)
