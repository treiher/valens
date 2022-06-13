from __future__ import annotations

import itertools
import sqlite3
from pathlib import Path

from alembic.command import revision, upgrade
from alembic.config import Config
from alembic.operations.ops import MigrationScript
from pytest_alembic.tests import (  # pylint: disable = unused-import
    test_model_definitions_match_ddl,
    test_single_head_revision,
    test_up_down_consistency,
    test_upgrade,
)

from valens import app, database as db

BASE_SCHEMA = Path("tests/data/base.sql")


def test_completeness(tmp_path: Path) -> None:
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

        db.init_db()
        connection = sqlite3.connect(models_db)

        model_constraints = constraints(connection)

        assert migrated_constraints == model_constraints