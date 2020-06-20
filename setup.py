import re

from setuptools import find_packages, setup  # type: ignore

with open("valens/__init__.py") as f:
    version = re.search(r'__version__ = "(.*?)"', f.read()).group(1)  # type: ignore

setup(
    name="valens",
    version=version,
    license="AGPL-3.0",
    packages=find_packages(where="valens"),
    python_requires=">=3.6, <4",
    install_requires=["matplotlib", "pandas"],
    entry_points={"console_scripts": ["valens=valens.valens:main"]},
)
