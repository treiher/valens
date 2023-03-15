import sqlite3

from sqlalchemy import select

import tests.data
from valens import database as db
from valens.models import User


def init_db_users() -> None:
    for user in tests.data.users_only():
        db.session.add(user)
        db.session.commit()


def init_db_data() -> None:
    for user in tests.data.users():
        db.session.add(user)
        db.session.commit()


def clear_db() -> None:
    for user in db.session.execute(select(User)).scalars():
        db.session.delete(user)
        db.session.commit()


def dump_db(connection: sqlite3.Connection) -> str:
    """
    Dump the database data with sorted constraints.

    The sorting of constraints is required, as alembic migrations that change some constraints
    lead to an indeterministic order of constraints.
    """
    unsorted_lines = [f"{l.rstrip()}\n" for s in connection.iterdump() for l in s.split("\n")]
    sorted_lines = []
    constraints = []

    for l in unsorted_lines:
        if l.startswith("\tCONSTRAINT "):
            constraints.append(l if l.endswith(",\n") else f"{l[:-1]},\n")
        else:
            if constraints:
                constraints.sort()
                constraints[-1] = f"{constraints[-1][:-2]}\n"
                sorted_lines.extend(constraints)
                constraints = []
            sorted_lines.append(l)

    return "".join(sorted_lines)
