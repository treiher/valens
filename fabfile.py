import subprocess
import sys
from pathlib import Path

from fabric import Connection, task  # type: ignore[import-untyped]


@task
def deploy(c: object, package: str | None = None, target_directory: str | None = None) -> None:
    if not isinstance(c, Connection):
        sys.exit("usage: fab -H user@host deploy [--package=FILE] [--target-directory=DIRECTORY]")

    if package:
        directory = str(Path(package).parent)
        filename = Path(package).name
    else:
        version = subprocess.check_output(["uv", "run", "hatch", "version"]).decode("utf-8").strip()
        directory = "dist"
        filename = f"valens-{version}-py3-none-any.whl"

    if not target_directory:
        target_directory = "/srv/http/valens"

    filepath = f"{target_directory}/{filename}"
    c.put(f"{directory}/{filename}", filepath)
    c.run(f"{target_directory}/venv/bin/pip install --force-reinstall {filepath}")
    c.run(f"rm {filepath}")
    c.run("systemctl restart uwsgi@valens", pty=True)
