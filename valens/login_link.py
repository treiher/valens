"""Login-link token handling shared by the API and the CLI."""

from __future__ import annotations

import hashlib
import secrets
from datetime import datetime, timedelta

from sqlalchemy.dialects.sqlite import insert as sqlite_insert

from valens import database as db
from valens.models import LoginLink

LIFETIME = timedelta(hours=24)


def create(user_id: int) -> str:
    """Create or replace the login-link token for the user and return the token."""
    token = secrets.token_urlsafe(32)
    expires_at = datetime.now() + LIFETIME
    db.session.execute(
        sqlite_insert(LoginLink)
        .values(user_id=user_id, token_hash=token_hash(token), expires_at=expires_at)
        .on_conflict_do_update(
            index_elements=[LoginLink.user_id],
            set_={"token_hash": token_hash(token), "expires_at": expires_at},
        )
    )
    return token


def token_hash(token: str) -> str:
    # The token is high-entropy, so a deterministic unsalted hash is sufficient and enables
    # the lookup by hash
    return hashlib.sha256(token.encode()).hexdigest()


def url(public_url: str, token: str) -> str:
    return f"{public_url.rstrip('/')}/login#recover={token}"
