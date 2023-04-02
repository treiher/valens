from __future__ import annotations

import itertools
import sqlite3
from collections.abc import Callable
from pathlib import Path

import pytest
from alembic.command import downgrade, revision, upgrade
from alembic.config import Config
from alembic.operations.ops import MigrationScript
from pytest_alembic.tests import (  # noqa: F401
    test_model_definitions_match_ddl,
    test_single_head_revision,
    test_up_down_consistency,
    test_upgrade,
)

from tests.utils import dump_db
from valens import app, database as db

DATA_DIR = Path("tests/data")
BASE_SCHEMA = DATA_DIR / "base.sql"


def assert_db_equality(
    tmp_path: Path, source: str, target: str, infix: str, migrate: Callable[[], None]
) -> None:
    test_db = tmp_path / "test.db"
    app.config["DATABASE"] = f"sqlite:///{test_db}"

    connection = sqlite3.connect(test_db)
    connection.executescript((DATA_DIR / f"{source}.sql").read_text(encoding="utf-8"))
    connection.commit()

    with app.app_context():
        migrate()
        filename = f"{target}_{infix}_{source}.sql"
        dump = dump_db(connection)
        (tmp_path / filename).write_text(dump)
        assert dump == (DATA_DIR / filename).read_text(encoding="utf-8")


def test_completeness(tmp_path: Path) -> None:
    """Ensure that all constraints defined in the model are added during the upgrade."""
    # Based on alembic-autogen-check (https://github.com/4Catalyzer/alembic-autogen-check)
    # The MIT License (MIT), Copyright (c) 2019 4Catalyzer

    cfg = Config("alembic.ini")
    test_db = f"/{tmp_path}/valens_test.db"
    app.config["DATABASE"] = f"sqlite://{test_db}"

    connection = sqlite3.connect(test_db)
    connection.executescript(BASE_SCHEMA.read_text(encoding="utf-8"))
    connection.commit()

    revisions = []

    def process_revision_directives(
        _context: object, _revision: object, directives: list[MigrationScript]
    ) -> None:
        nonlocal revisions
        revisions = list(directives)
        # Prevent actually generating a migration
        directives[:] = []

    with app.app_context():
        upgrade(cfg, "head")
        revision(cfg, autogenerate=True, process_revision_directives=process_revision_directives)
        diff = list(
            itertools.chain.from_iterable(
                op.as_diffs() for script in revisions for op in script.upgrade_ops_list
            )
        )

        assert not diff, "some model changes are missing in migrations"


def test_completeness_constraints(tmp_path: Path) -> None:
    """Ensure that all constraints defined in the model are added during the upgrade."""

    def constraints(connection: sqlite3.Connection) -> list[str]:
        return sorted(
            [
                l.strip()[:-1] if l.strip().endswith(",") else l.strip()
                for l in "\n".join(list(connection.iterdump())).split("\n")
                if "CONSTRAINT" in l
            ]
        )

    with app.app_context():
        migrations_db = f"{tmp_path}/migrations.db"
        app.config["DATABASE"] = f"sqlite:///{migrations_db}"

        connection = sqlite3.connect(migrations_db)
        connection.executescript(BASE_SCHEMA.read_text(encoding="utf-8"))
        connection.commit()
        upgrade(Config("alembic.ini"), "head")

        migrated_constraints = constraints(connection)

        models_db = f"{tmp_path}/models.db"
        app.config["DATABASE"] = f"sqlite:///{models_db}"

        db.init()
        connection = sqlite3.connect(models_db)

        model_constraints = constraints(connection)

        assert migrated_constraints == model_constraints


@pytest.mark.parametrize(
    ("source", "target"),
    [
        ("4b6051594962", "b9f4e42c7135"),
        ("b9f4e42c7135", "8a0dc258bf2a"),
        ("8a0dc258bf2a", "22f3ddb25741"),
        ("22f3ddb25741", "06f82ead211b"),
    ],
)
def test_up(tmp_path: Path, source: str, target: str) -> None:
    assert_db_equality(
        tmp_path, source, target, "up_from", lambda: upgrade(Config("alembic.ini"), target)
    )


@pytest.mark.parametrize(
    ("source", "target"),
    [
        ("b9f4e42c7135", "4b6051594962"),
        ("8a0dc258bf2a", "b9f4e42c7135"),
        ("22f3ddb25741", "8a0dc258bf2a"),
        ("06f82ead211b", "22f3ddb25741"),
    ],
)
def test_down(tmp_path: Path, source: str, target: str) -> None:
    assert_db_equality(
        tmp_path, source, target, "down_from", lambda: downgrade(Config("alembic.ini"), target)
    )
