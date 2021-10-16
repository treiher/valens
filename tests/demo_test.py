from pathlib import Path

from pytest import MonkeyPatch

from valens import demo, web


def test_run(tmp_path: Path, monkeypatch: MonkeyPatch) -> None:
    monkeypatch.setattr(web.app, "run", lambda x: None)
    demo.run(f"sqlite:///{tmp_path}/db")
