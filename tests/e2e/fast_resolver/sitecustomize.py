"""Site customization for servers started by end-to-end tests."""

import socket


def _getfqdn(name: str = "") -> str:
    return name


# On GitHub-hosted macOS runners, the reverse lookup performed by `socket.getfqdn` blocks for 35 s.
socket.getfqdn = _getfqdn
