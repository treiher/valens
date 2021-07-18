import sqlite3

from alembic import command
from alembic.config import Config
from flask import g
from sqlalchemy import MetaData, create_engine, event, pool
from sqlalchemy.engine import Engine
from sqlalchemy.orm import Session, declarative_base, scoped_session, sessionmaker
from sqlalchemy_repr import RepresentableBase
from werkzeug.local import LocalProxy

from valens import app

meta = MetaData(
    naming_convention={
        "ix": "ix_%(column_0_label)s",
        "uq": "uq_%(table_name)s_%(column_0_name)s",
        "ck": "ck_%(table_name)s_%(constraint_name)s",
        "fk": "fk_%(table_name)s_%(column_0_name)s_%(referred_table_name)s",
        "pk": "pk_%(table_name)s",
    }
)

Base = declarative_base(cls=RepresentableBase, metadata=meta)

alembic_cfg = Config()
alembic_cfg.set_main_option("script_location", "valens:migrations")


@event.listens_for(Engine, "connect")
def _set_sqlite_pragma(
    dbapi_connection: sqlite3.Connection,
    _: pool.base._ConnectionRecord,  # pylint: disable = protected-access
) -> None:
    if app.config["SQLITE_FOREIGN_KEY_SUPPORT"]:
        cursor = dbapi_connection.cursor()
        cursor.execute("PRAGMA foreign_keys=ON")
        cursor.close()


def get_engine() -> Engine:
    return create_engine(app.config["DATABASE"])


def get_scoped_session() -> scoped_session:
    return scoped_session(
        sessionmaker(autocommit=False, autoflush=False, bind=get_engine(), future=True)
    )


def get_session() -> Session:
    if "db_session" not in g:
        # ISSUE: PyCQA/pylint#3793
        g.db_session = get_scoped_session()()  # pylint: disable = assigning-non-slot

    return g.db_session


session: Session = LocalProxy(get_session)  # type: ignore


def remove_session() -> None:
    get_scoped_session().remove()


def init_db() -> None:
    import valens.models  # pylint: disable = unused-import, import-outside-toplevel

    Base.query = get_scoped_session().query_property()
    Base.metadata.create_all(bind=get_engine())

    command.stamp(alembic_cfg, "head")


def upgrade_db() -> None:
    command.upgrade(alembic_cfg, "head")
