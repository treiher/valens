#!/usr/bin/env python

"""Create a new database containing the data defined in `tests/data.py`."""

import sys
from pathlib import Path

from tests.utils import init_db_data
from valens import app

db_file = Path(app.config["DATABASE"].removeprefix("sqlite:///"))

if db_file.exists():
    sys.exit(f"'{db_file}' already exists")

with app.app_context():
    init_db_data()
