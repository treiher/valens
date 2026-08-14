"""End-to-end tests for passkey authentication and login-link account recovery."""

from __future__ import annotations

import os
from base64 import b64encode
from collections.abc import Generator
from pathlib import Path
from subprocess import PIPE, STDOUT, Popen, run
from tempfile import TemporaryDirectory

import pytest
from cryptography.hazmat.primitives.asymmetric.ec import SECP256R1, generate_private_key
from cryptography.hazmat.primitives.serialization import (
    Encoding,
    NoEncryption,
    PrivateFormat,
)
from playwright.sync_api import CDPSession, Page, expect

import tests.utils
from valens import app
from valens.config import create_config_file

from .const import PORT, TODAY, USERNAMES, VALENS
from .io import wait_for_output
from .pages import AdminDialog, HomePage, LoginPage, PasskeyRegistrationView, ProfileDialog

# WebAuthn requires a valid relying party ID, which an IP address is not
AUTH_BASE_URL = f"http://localhost:{PORT}"


@pytest.fixture(autouse=True)
def backend_server(request: pytest.FixtureRequest) -> Generator[Path, None, None]:
    """Start the backend server with a fresh database for each test."""

    param = getattr(request, "param", {})
    username_login_enabled = param.get("username_login_enabled", True)
    public_url_set = param.get("public_url_set", True)

    with TemporaryDirectory() as tmp_dir:
        data_dir = Path(tmp_dir)
        db_file = data_dir / "test.db"
        config = create_config_file(data_dir, db_file)
        config.write_text(
            "".join(
                line
                for line in config.read_text().splitlines(keepends=True)
                if not line.startswith("PUBLIC_URL")
            )
            + (f"PUBLIC_URL = '{AUTH_BASE_URL}'\n" if public_url_set else "")
            + ("" if username_login_enabled else "USERNAME_LOGIN_ENABLED = False\n")
        )

        with app.app_context():
            app.config["DATABASE"] = f"sqlite:///{db_file}"
            app.config["SECRET_KEY"] = b"TEST_KEY"
            tests.utils.init_db_data(today=TODAY)

        with Popen(
            f"{VALENS} run --port {PORT}".split(),
            stdout=PIPE,
            stderr=STDOUT,
            env={"VALENS_CONFIG": str(config), **os.environ},
        ) as p:
            assert p.stdout
            wait_for_output(p.stdout, "Running on")
            yield config
            p.terminate()


@pytest.fixture(autouse=True)
def virtual_authenticator(page: Page) -> tuple[CDPSession, str]:
    """Add a virtual authenticator with a user-verifying platform authenticator."""

    client = page.context.new_cdp_session(page)
    client.send("WebAuthn.enable")
    authenticator = client.send(
        "WebAuthn.addVirtualAuthenticator",
        {
            "options": {
                "protocol": "ctap2",
                "transport": "internal",
                "hasResidentKey": True,
                "hasUserVerification": True,
                "isUserVerified": True,
                "automaticPresenceSimulation": True,
            }
        },
    )
    return client, authenticator["authenticatorId"]


def login(page: Page, username: str = USERNAMES[0]) -> None:
    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    login_page.goto()
    login_page.login(username)


def test_passkey_registration_and_login(page: Page) -> None:
    login(page)

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.expect_passkeys([])

    profile_dialog.add_passkey()
    profile_dialog.expect_passkeys(["Passkey"])
    profile_dialog.close()

    home_page = HomePage(page, base_url=AUTH_BASE_URL)
    home_page.navbar.logout()

    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    login_page.expect_page()
    login_page.login_with_passkey()

    home_page.expect_page()


def test_passkey_login_with_unverifiable_signature(
    page: Page, virtual_authenticator: tuple[CDPSession, str]
) -> None:
    login(page)

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.add_passkey()
    profile_dialog.close()

    home_page = HomePage(page, base_url=AUTH_BASE_URL)
    home_page.navbar.logout()

    replace_credential_key_pair(*virtual_authenticator)

    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    login_page.expect_page()
    login_page.click_passkey_login()

    expect(login_page.error_message).to_have_text("Passkey could not be verified")


def replace_credential_key_pair(client: CDPSession, authenticator_id: str) -> None:
    """Give the registered credential a new key pair, keeping its credential ID."""

    credentials = client.send("WebAuthn.getCredentials", {"authenticatorId": authenticator_id})
    credential = credentials["credentials"][0]
    private_key = generate_private_key(SECP256R1()).private_bytes(
        Encoding.DER, PrivateFormat.PKCS8, NoEncryption()
    )
    client.send(
        "WebAuthn.removeCredential",
        {"authenticatorId": authenticator_id, "credentialId": credential["credentialId"]},
    )
    client.send(
        "WebAuthn.addCredential",
        {
            "authenticatorId": authenticator_id,
            "credential": {
                "credentialId": credential["credentialId"],
                "isResidentCredential": True,
                "rpId": credential["rpId"],
                "userHandle": credential["userHandle"],
                "privateKey": b64encode(private_key).decode("ascii"),
                "signCount": credential["signCount"],
            },
        },
    )


def test_passkey_rename_and_delete(page: Page) -> None:
    login(page)

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.add_passkey()

    profile_dialog.rename_passkey(0, "My Passkey")
    profile_dialog.expect_passkeys(["My Passkey"])

    profile_dialog.delete_passkey(0)
    profile_dialog.expect_passkeys([])


def test_admin_passkey_deletion(page: Page) -> None:
    login(page, USERNAMES[1])

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.add_passkey()
    profile_dialog.close()

    home_page = HomePage(page, base_url=AUTH_BASE_URL)
    home_page.navbar.logout()

    login(page, USERNAMES[0])

    admin_dialog = AdminDialog(page)
    admin_dialog.open()
    admin_dialog.open_user_passkeys(USERNAMES[1])
    admin_dialog.expect_user_passkeys(1)
    admin_dialog.delete_user_passkey(0)
    admin_dialog.expect_user_passkeys(0)


def test_login_link_login(page: Page) -> None:
    login(page)

    admin_dialog = AdminDialog(page)
    admin_dialog.open()
    admin_dialog.expect_no_passkey_login_unavailable_info()
    url = admin_dialog.create_login_link(USERNAMES[1])

    assert url.startswith(f"{AUTH_BASE_URL}/login#recover=")

    home_page = HomePage(page, base_url=AUTH_BASE_URL)
    page.goto(url)
    home_page.expect_page()

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.expect_name(USERNAMES[1])
    profile_dialog.close()

    home_page.navbar.logout()

    # The link is invalidated on use
    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    # Signing out leads to the login page, so opening the link only changes the URL fragment
    login_page.expect_page()
    page.goto(url)
    login_page.expect_page()
    expect(login_page.error_message).to_have_text("The login link is invalid or has expired")


@pytest.mark.parametrize("backend_server", [{"public_url_set": False}], indirect=True)
def test_passkey_login_and_login_link_unavailable_without_public_url(page: Page) -> None:
    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    login_page.goto()
    login_page.expect_username_login_only()
    login_page.login(USERNAMES[0])

    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.expect_no_passkey_section()
    profile_dialog.close()

    # Login links also require `PUBLIC_URL`
    admin_dialog = AdminDialog(page)
    admin_dialog.open()
    admin_dialog.expect_passkey_login_unavailable_info()
    admin_dialog.expect_no_login_link_option(USERNAMES[1])


@pytest.mark.parametrize("backend_server", [{"username_login_enabled": False}], indirect=True)
def test_forced_passkey_registration_with_disabled_username_login(
    backend_server: Path, page: Page
) -> None:
    p = run(
        f"{VALENS} user login-link {USERNAMES[0]}".split(),
        check=False,
        stdout=PIPE,
        stderr=STDOUT,
        env={"VALENS_CONFIG": str(backend_server), **os.environ},
    )
    assert p.returncode == 0
    url = p.stdout.decode("utf-8").strip()
    assert url.startswith(f"{AUTH_BASE_URL}/login#recover=")

    page.goto(url)

    # Without a passkey, all routes redirect to the passkey registration view
    registration_view = PasskeyRegistrationView(page)
    registration_view.expect_view()
    page.goto(f"{AUTH_BASE_URL}/body_weight")
    registration_view.expect_view()

    page.goto(f"{AUTH_BASE_URL}/home")
    registration_view.expect_view()
    registration_view.register_passkey()
    HomePage(page, base_url=AUTH_BASE_URL).expect_page()

    # The last passkey must not be deletable while username login is disabled
    profile_dialog = ProfileDialog(page)
    profile_dialog.open()
    profile_dialog.expect_passkey_deletion_blocked(0)
    profile_dialog.close()

    home_page = HomePage(page, base_url=AUTH_BASE_URL)
    home_page.navbar.logout()

    login_page = LoginPage(page, base_url=AUTH_BASE_URL)
    login_page.expect_passkey_login_only()
    login_page.login_with_passkey()

    home_page.expect_page()
