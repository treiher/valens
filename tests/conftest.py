from __future__ import annotations

import pytest

from valens import app, database as db


@pytest.fixture
def alembic_config() -> dict[str, str]:
    return {"script_location": "valens:migrations"}


@pytest.fixture
def alembic_engine() -> object:
    with app.app_context():
        app.config["DATABASE"] = "sqlite:///"
        yield db.get_engine()
