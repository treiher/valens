from importlib.metadata import version as package_version
from pathlib import Path

import pytest

from valens import version


@pytest.fixture(name="version_file")
def fixture_version_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    path = tmp_path / "version"
    monkeypatch.setattr(version, "BUILD_VERSION_FILE", path)
    return path


def test_build_version(version_file: Path) -> None:
    version_file.write_text("1.2.3.dev4+g0123456\n")
    assert version.get() == "1.2.3.dev4+g0123456"


def test_build_version_read_on_each_call(version_file: Path) -> None:
    version_file.write_text("1.2.3")
    assert version.get() == "1.2.3"
    version_file.write_text("1.2.4")
    assert version.get() == "1.2.4"


@pytest.mark.usefixtures("version_file")
def test_missing_build_version() -> None:
    assert version.get() == package_version("valens")


@pytest.mark.parametrize("content", ["", "\n"])
def test_empty_build_version(version_file: Path, content: str) -> None:
    version_file.write_text(content)
    assert version.get() == package_version("valens")
