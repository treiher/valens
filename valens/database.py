from flask import g
from sqlalchemy import create_engine
from sqlalchemy.engine import Engine
from sqlalchemy.orm import Session, declarative_base, scoped_session, sessionmaker
from sqlalchemy_repr import RepresentableBase
from werkzeug.local import LocalProxy

from valens import app

Base = declarative_base(cls=RepresentableBase)


def get_engine() -> Engine:
    return create_engine(app.config["DATABASE"])


def get_scoped_session() -> scoped_session:
    return scoped_session(sessionmaker(autocommit=False, autoflush=False, bind=get_engine()))


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
