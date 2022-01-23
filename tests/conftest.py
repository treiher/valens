from __future__ import annotations

from tempfile import NamedTemporaryFile

import pytest

from valens import app, database as db


@pytest.fixture
def alembic_config() -> dict[str, str]:
    return {"script_location": "valens:migrations"}


@pytest.fixture
def alembic_engine() -> object:
    with NamedTemporaryFile() as tmp_file:
        app.config["DATABASE"] = f"sqlite:///{tmp_file.name}"
        with app.app_context():
            db.init_db()
            yield db.get_engine()
