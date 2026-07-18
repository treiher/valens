from __future__ import annotations

import hashlib
import json
import logging
import os
import struct
import uuid
from collections.abc import Generator
from datetime import datetime, timedelta
from http import HTTPStatus
from pathlib import Path

import pytest
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature
from sqlalchemy import select
from webauthn.helpers import base64url_to_bytes, bytes_to_base64url, encode_cbor
from werkzeug.test import Client, TestResponse as Response

import tests.utils
from valens import app, database as db, login_link
from valens.models import LoginLink, Passkey

PUBLIC_URL = "http://localhost:5000"

GOOGLE_PASSWORD_MANAGER_AAGUID = "ea9b8d66-4d01-1d21-3ce4-b6b48cb575d4"


class SoftAuthenticator:
    """A minimal software authenticator producing real WebAuthn responses."""

    def __init__(self, aaguid: str = "00000000-0000-0000-0000-000000000000") -> None:
        self.rp_id = "localhost"
        self.origin = PUBLIC_URL
        self.aaguid = uuid.UUID(aaguid).bytes
        self.credential_id = os.urandom(32)
        self.private_key = ec.generate_private_key(ec.SECP256R1())
        self.sign_count = 0

    def create(self, options: dict[str, object]) -> dict[str, object]:
        client_data = self._client_data("webauthn.create", options)
        numbers = self.private_key.public_key().public_numbers()
        cose_public_key = encode_cbor(
            {
                1: 2,
                3: -7,
                -1: 1,
                -2: numbers.x.to_bytes(32, "big"),
                -3: numbers.y.to_bytes(32, "big"),
            }
        )
        auth_data = (
            self._rp_id_hash()
            # Flags: user present, user verified, attested credential data included
            + b"\x45"
            + struct.pack(">I", self.sign_count)
            + self.aaguid
            + struct.pack(">H", len(self.credential_id))
            + self.credential_id
            + cose_public_key
        )
        attestation_object = encode_cbor({"fmt": "none", "attStmt": {}, "authData": auth_data})
        return {
            "id": bytes_to_base64url(self.credential_id),
            "rawId": bytes_to_base64url(self.credential_id),
            "response": {
                "clientDataJSON": bytes_to_base64url(client_data),
                "attestationObject": bytes_to_base64url(attestation_object),
                "transports": ["internal"],
            },
            "type": "public-key",
            "clientExtensionResults": {},
        }

    def get(self, options: dict[str, object]) -> dict[str, object]:
        client_data = self._client_data("webauthn.get", options)
        # Flags: user present, user verified
        auth_data = self._rp_id_hash() + b"\x05" + struct.pack(">I", self.sign_count)
        signature = self.private_key.sign(
            auth_data + hashlib.sha256(client_data).digest(), ec.ECDSA(hashes.SHA256())
        )
        return {
            "id": bytes_to_base64url(self.credential_id),
            "rawId": bytes_to_base64url(self.credential_id),
            "response": {
                "clientDataJSON": bytes_to_base64url(client_data),
                "authenticatorData": bytes_to_base64url(auth_data),
                "signature": bytes_to_base64url(signature),
                "userHandle": None,
            },
            "type": "public-key",
            "clientExtensionResults": {},
        }

    def _client_data(self, type_: str, options: dict[str, object]) -> bytes:
        return json.dumps(
            {
                "type": type_,
                "challenge": options["challenge"],
                "origin": self.origin,
                "crossOrigin": False,
            }
        ).encode()

    def _rp_id_hash(self) -> bytes:
        return hashlib.sha256(self.rp_id.encode()).digest()


@pytest.fixture(name="client")
def fixture_client(tmp_path: Path) -> Generator[Client, None, None]:
    app.config["DATABASE"] = f"sqlite:///{tmp_path}/valens.db"
    app.config["SECRET_KEY"] = b"TEST_KEY"
    app.config["TESTING"] = True
    app.config["PUBLIC_URL"] = PUBLIC_URL

    with app.test_client() as client, app.app_context():
        yield client

    del app.config["PUBLIC_URL"]
    app.config["USERNAME_LOGIN_ENABLED"] = True


def create_session(client: Client, name: str = "Alice") -> Response:
    return client.post("/api/session", json={"name": name})


def register_passkey(
    client: Client, authenticator: SoftAuthenticator | None = None
) -> tuple[SoftAuthenticator, Response]:
    authenticator = authenticator or SoftAuthenticator()
    resp = client.post("/api/auth/passkeys/registration/options")
    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    return (
        authenticator,
        client.post("/api/auth/passkeys/registration", json=authenticator.create(resp.json)),
    )


def authenticate_passkey(client: Client, authenticator: SoftAuthenticator) -> Response:
    resp = client.post("/api/auth/passkeys/authentication/options")
    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    return client.post("/api/auth/passkeys/authentication", json=authenticator.get(resp.json))


def test_read_auth(client: Client) -> None:
    resp = client.get("/api/auth")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"methods": ["passkey", "username"]}

    app.config["USERNAME_LOGIN_ENABLED"] = False

    resp = client.get("/api/auth")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json == {"methods": ["passkey"]}

    app.config["USERNAME_LOGIN_ENABLED"] = True
    del app.config["PUBLIC_URL"]
    try:
        resp = client.get("/api/auth")

        assert resp.status_code == HTTPStatus.OK
        assert resp.json == {"methods": ["username"]}
    finally:
        app.config["PUBLIC_URL"] = PUBLIC_URL


def test_create_session_username_login_disabled(client: Client) -> None:
    tests.utils.init_db_users()

    app.config["USERNAME_LOGIN_ENABLED"] = False

    resp = create_session(client)

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "username login is disabled"}


def test_registration_and_authentication(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["rp"] == {"id": "localhost", "name": "Valens"}
    assert resp.json["authenticatorSelection"]["residentKey"] == "required"
    assert resp.json["authenticatorSelection"]["userVerification"] == "required"
    assert resp.json["attestation"] == "none"
    assert resp.json["excludeCredentials"] == []

    authenticator = SoftAuthenticator()
    resp = client.post("/api/auth/passkeys/registration", json=authenticator.create(resp.json))

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]
    assert resp.json["label"] == "Passkey"
    assert resp.json["last_used"] is None

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["excludeCredentials"] == [
        {
            "id": bytes_to_base64url(authenticator.credential_id),
            "type": "public-key",
        }
    ]

    assert client.delete("/api/session").status_code == HTTPStatus.NO_CONTENT

    resp = client.post("/api/auth/passkeys/authentication/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["rpId"] == "localhost"
    assert resp.json["userVerification"] == "required"
    assert "allowCredentials" not in resp.json or resp.json["allowCredentials"] == []

    resp = client.post("/api/auth/passkeys/authentication", json=authenticator.get(resp.json))

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["name"] == "Alice"

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["name"] == "Alice"

    resp = client.get("/api/users/1/passkeys")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json[0]["id"] == passkey_id
    assert resp.json[0]["last_used"] is not None


def test_registration_infers_label_from_aaguid(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    _, resp = register_passkey(client, SoftAuthenticator(aaguid=GOOGLE_PASSWORD_MANAGER_AAGUID))

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    assert resp.json["label"] == "Google Password Manager"


def test_registration_conflict(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    authenticator, resp = register_passkey(client)

    assert resp.status_code == HTTPStatus.CREATED

    _, resp = register_passkey(client, authenticator)

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "passkey is already registered"}


@pytest.mark.parametrize(
    ("transports", "expected"),
    [
        (["internal", "invalid", 1], "internal"),
        ("internal", ""),
    ],
)
def test_registration_sanitizes_transports(
    client: Client, transports: object, expected: str
) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None

    credential = SoftAuthenticator().create(resp.json)
    response = credential["response"]
    assert isinstance(response, dict)
    response["transports"] = transports

    resp = client.post("/api/auth/passkeys/registration", json=credential)

    assert resp.status_code == HTTPStatus.CREATED
    assert db.session.execute(select(Passkey)).scalars().one().transports == expected


def test_registration_without_options(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration", json={})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": "no passkey registration in progress"}


def test_registration_invalid_response(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration", json={"id": "MQ"})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json is not None
    assert "details" in resp.json


def test_registration_options_user_deleted(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    tests.utils.clear_db()

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert not resp.data


def test_registration_user_deleted(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/registration/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None

    options = resp.json

    tests.utils.clear_db()

    resp = client.post("/api/auth/passkeys/registration", json=SoftAuthenticator().create(options))

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert not resp.data


def test_authentication_without_options(client: Client) -> None:
    resp = client.post("/api/auth/passkeys/authentication", json={})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": "no passkey authentication in progress"}


def test_authentication_invalid_raw_id(client: Client) -> None:
    assert client.post("/api/auth/passkeys/authentication/options").status_code == HTTPStatus.OK

    resp = client.post("/api/auth/passkeys/authentication", json={"rawId": 1})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": "rawId must be a string"}


def test_authentication_unknown_credential(client: Client) -> None:
    tests.utils.init_db_users()

    authenticator = SoftAuthenticator()
    resp = client.post("/api/auth/passkeys/authentication/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None

    resp = client.post("/api/auth/passkeys/authentication", json=authenticator.get(resp.json))

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_authentication_malformed_credential(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    authenticator, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert client.delete("/api/session").status_code == HTTPStatus.NO_CONTENT

    assert client.post("/api/auth/passkeys/authentication/options").status_code == HTTPStatus.OK

    resp = client.post(
        "/api/auth/passkeys/authentication",
        json={"rawId": bytes_to_base64url(authenticator.credential_id)},
    )

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert resp.json == {"details": "Credential missing required id"}


def test_authentication_with_raw_signature(
    client: Client, caplog: pytest.LogCaptureFixture
) -> None:
    """Signatures which are not DER-encoded are rejected and logged with their size."""

    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    authenticator, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert client.delete("/api/session").status_code == HTTPStatus.NO_CONTENT

    resp = client.post("/api/auth/passkeys/authentication/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None

    credential = authenticator.get(resp.json)
    assert isinstance(credential["response"], dict)
    r, s = decode_dss_signature(base64url_to_bytes(str(credential["response"]["signature"])))
    credential["response"]["signature"] = bytes_to_base64url(
        r.to_bytes(32, "big") + s.to_bytes(32, "big")
    )

    with caplog.at_level(logging.WARNING):
        resp = client.post("/api/auth/passkeys/authentication", json=credential)

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert resp.json == {"details": "Could not verify authentication signature"}
    assert "failed verification of passkey 1 of user 1" in caplog.text
    assert "signature of 64 bytes" in caplog.text


def test_authentication_invalid_response(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    authenticator, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert client.delete("/api/session").status_code == HTTPStatus.NO_CONTENT

    resp = client.post("/api/auth/passkeys/authentication/options")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None

    credential = authenticator.get({**resp.json, "challenge": bytes_to_base64url(os.urandom(32))})
    resp = client.post("/api/auth/passkeys/authentication", json=credential)

    assert resp.status_code == HTTPStatus.UNAUTHORIZED
    assert resp.json is not None
    assert "details" in resp.json

    resp = client.get("/api/session")

    assert resp.status_code == HTTPStatus.NOT_FOUND


def test_authentication_sign_count_regression(
    client: Client, caplog: pytest.LogCaptureFixture
) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    authenticator = SoftAuthenticator()
    authenticator.sign_count = 5
    _, resp = register_passkey(client, authenticator)
    assert resp.status_code == HTTPStatus.CREATED
    assert client.delete("/api/session").status_code == HTTPStatus.NO_CONTENT

    authenticator.sign_count = 6

    with caplog.at_level(logging.WARNING):
        assert authenticate_passkey(client, authenticator).status_code == HTTPStatus.OK

    assert "sign count regression" not in caplog.text

    authenticator.sign_count = 3

    with caplog.at_level(logging.WARNING):
        assert authenticate_passkey(client, authenticator).status_code == HTTPStatus.OK

    assert "sign count regression" in caplog.text


@pytest.mark.parametrize(
    "route",
    [
        "/api/auth/passkeys/registration/options",
        "/api/auth/passkeys/registration",
        "/api/auth/passkeys/authentication/options",
        "/api/auth/passkeys/authentication",
        "/api/auth/login-link",
    ],
)
def test_public_url_not_set(client: Client, route: str) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    del app.config["PUBLIC_URL"]
    try:
        resp = client.post(route, json={})

        assert resp.status_code == HTTPStatus.INTERNAL_SERVER_ERROR
        assert resp.json == {"details": "'PUBLIC_URL' is not set in app config"}
    finally:
        app.config["PUBLIC_URL"] = PUBLIC_URL


@pytest.mark.parametrize("public_url", [PUBLIC_URL, f"{PUBLIC_URL}/"])
def test_login_link_url(public_url: str) -> None:
    assert login_link.url(public_url, "token") == f"{PUBLIC_URL}/login#recover=token"


def test_login_link_lifecycle(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/login-link", json={"user_id": 2})

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    first_token = resp.json["url"].removeprefix(f"{PUBLIC_URL}/login#recover=")
    assert first_token
    assert first_token != resp.json["url"]

    link = db.session.execute(select(LoginLink)).scalars().one()
    assert link.user_id == 2
    assert first_token not in link.token_hash
    assert link.token_hash == hashlib.sha256(first_token.encode()).hexdigest()

    # A new link replaces the previous one
    resp = client.post("/api/auth/login-link", json={"user_id": 2})

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    second_token = resp.json["url"].removeprefix(f"{PUBLIC_URL}/login#recover=")
    assert db.session.execute(select(LoginLink)).scalars().one().user_id == 2

    other_client = app.test_client()
    resp = other_client.post("/api/auth/session", json={"token": first_token})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data

    resp = other_client.post("/api/auth/session", json={"token": second_token})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["name"] == "Bob"

    resp = other_client.get("/api/session")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["name"] == "Bob"

    # The link is deleted on use
    resp = other_client.post("/api/auth/session", json={"token": second_token})

    assert resp.status_code == HTTPStatus.NOT_FOUND


def test_login_link_expired(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/login-link", json={"user_id": 2})

    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    token = resp.json["url"].removeprefix(f"{PUBLIC_URL}/login#recover=")

    link = db.session.execute(select(LoginLink)).scalars().one()
    link.expires_at = datetime.now() - timedelta(minutes=1)
    db.session.commit()

    resp = client.post("/api/auth/session", json={"token": token})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data
    assert db.session.execute(select(LoginLink)).scalars().one_or_none() is None


def test_login_link_forbidden(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.post("/api/auth/login-link", json={"user_id": 2})

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_login_link_invalid_user_id(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/login-link", json={"user_id": "2"})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": "user_id must be an integer"}


def test_login_link_unknown_user(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.post("/api/auth/login-link", json={"user_id": 3})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_login_link_redemption_invalid_token(client: Client) -> None:
    resp = client.post("/api/auth/session", json={"token": 1})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": "token must be a string"}


def test_login_link_redemption_unknown_token(client: Client) -> None:
    resp = client.post("/api/auth/session", json={"token": "unknown"})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_read_passkeys_forbidden(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.get("/api/users/1/passkeys")

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_read_passkeys_unknown_user(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/users/3/passkeys")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_read_passkeys_of_other_user_as_admin(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.get("/api/users/2/passkeys")

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert len(resp.json) == 1


def test_update_passkey(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]

    resp = client.patch(f"/api/users/1/passkeys/{passkey_id}", json={"label": " My Passkey "})

    assert resp.status_code == HTTPStatus.OK
    assert resp.json is not None
    assert resp.json["label"] == "My Passkey"

    passkey = db.session.execute(select(Passkey)).scalars().one()
    assert passkey.label == "My Passkey"


@pytest.mark.parametrize(
    ("label", "details"),
    [
        (1, "label must be a string"),
        ("  ", "label must not be empty"),
        ("x" * 65, "label must be 64 characters or fewer"),
    ],
)
def test_update_passkey_invalid_label(client: Client, label: object, details: str) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None

    resp = client.patch(f"/api/users/1/passkeys/{resp.json['id']}", json={"label": label})

    assert resp.status_code == HTTPStatus.BAD_REQUEST
    assert resp.json == {"details": details}


def test_update_passkey_forbidden(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.patch("/api/users/1/passkeys/1", json={"label": "X"})

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_update_passkey_not_found(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.patch("/api/users/1/passkeys/1", json={"label": "X"})

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data


def test_delete_passkey(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]

    resp = client.delete(f"/api/users/1/passkeys/{passkey_id}")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert not resp.data
    assert db.session.execute(select(Passkey)).scalars().one_or_none() is None


def test_delete_passkey_of_other_user_as_admin(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.delete(f"/api/users/2/passkeys/{passkey_id}")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert db.session.execute(select(Passkey)).scalars().one_or_none() is None


def test_delete_last_passkey_username_login_disabled(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]

    app.config["USERNAME_LOGIN_ENABLED"] = False

    resp = client.delete(f"/api/users/1/passkeys/{passkey_id}")

    assert resp.status_code == HTTPStatus.CONFLICT
    assert resp.json == {"details": "last passkey cannot be deleted"}
    assert db.session.execute(select(Passkey)).scalars().one_or_none() is not None

    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED

    # A passkey that is not the last one can be deleted
    resp = client.delete(f"/api/users/1/passkeys/{passkey_id}")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert db.session.execute(select(Passkey)).scalars().one_or_none() is not None


def test_delete_last_passkey_of_other_user_as_admin_username_login_disabled(
    client: Client,
) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK
    _, resp = register_passkey(client)
    assert resp.status_code == HTTPStatus.CREATED
    assert resp.json is not None
    passkey_id = resp.json["id"]

    assert create_session(client).status_code == HTTPStatus.OK

    app.config["USERNAME_LOGIN_ENABLED"] = False

    resp = client.delete(f"/api/users/2/passkeys/{passkey_id}")

    assert resp.status_code == HTTPStatus.NO_CONTENT
    assert db.session.execute(select(Passkey)).scalars().one_or_none() is None


def test_delete_passkey_forbidden(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client, "Bob").status_code == HTTPStatus.OK

    resp = client.delete("/api/users/1/passkeys/1")

    assert resp.status_code == HTTPStatus.FORBIDDEN
    assert resp.json == {"details": "user is not an administrator"}


def test_delete_passkey_not_found(client: Client) -> None:
    tests.utils.init_db_users()

    assert create_session(client).status_code == HTTPStatus.OK

    resp = client.delete("/api/users/1/passkeys/1")

    assert resp.status_code == HTTPStatus.NOT_FOUND
    assert not resp.data
