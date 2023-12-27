#!/usr/bin/env python

"""Dump the database data in the same format as in `tests/data/*.sql`."""

import sqlite3
from pathlib import Path

from tests.utils import dump_db
from valens import app

db_file = Path(app.config["DATABASE"].removeprefix("sqlite:///"))
connection = sqlite3.connect(db_file)

print(dump_db(connection), end="")  # noqa: T201
