from pathlib import Path

import pytest

from valens import app, demo


def test_run(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(app, "run", lambda x, y: None)
    demo.run(f"sqlite:///{tmp_path}/db")
