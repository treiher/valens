import sys
from pathlib import Path
from typing import Optional

from fabric import Connection, task  # type: ignore[import]
from setuptools_scm import get_version  # type: ignore[import]


@task
def deploy(
    c: object, package: Optional[str] = None, target_directory: Optional[str] = None
) -> None:
    if not isinstance(c, Connection):
        sys.exit("usage: fab -H user@host deploy")

    if package:
        directory = str(Path(package).parent)
        filename = Path(package).name
    else:
        directory = "dist"
        filename = f"valens-{get_version()}-py3-none-any.whl"

    if not target_directory:
        target_directory = "/srv/http/valens"

    filepath = f"{target_directory}/{filename}"
    c.put(f"{directory}/{filename}", filepath)
    c.run(f"{target_directory}/venv/bin/pip install --force-reinstall {filepath}")
    c.run(f"rm {filepath}")
    c.run("systemctl restart uwsgi@valens", pty=True)
