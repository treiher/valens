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
