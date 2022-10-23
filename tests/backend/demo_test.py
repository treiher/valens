from pathlib import Path

from pytest import MonkeyPatch

from valens import app, demo


def test_run(tmp_path: Path, monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(app, "run", lambda x, y: None)
    demo.run(f"sqlite:///{tmp_path}/db")
