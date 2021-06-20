import tests.data
from valens import database as db


def init_db_users() -> None:
    for user in tests.data.users_only():
        db.session.add(user)
        db.session.commit()


def init_db_data() -> None:
    for user in tests.data.users():
        db.session.add(user)
        db.session.commit()
