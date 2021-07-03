from pathlib import Path

from valens import app, database as db


def test_init_upgrade(tmp_path: Path) -> None:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["TESTING"] = True

    with app.app_context():
        db.init_db()
        db.upgrade_db()
