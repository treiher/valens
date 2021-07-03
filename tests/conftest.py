from __future__ import annotations

import pytest


@pytest.fixture
def alembic_config() -> dict[str, str]:
    return {"script_location": "valens:migrations"}
