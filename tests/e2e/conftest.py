from __future__ import annotations

from collections.abc import Generator

import pytest
from playwright.sync_api import BrowserContext, Page


@pytest.fixture(autouse=True)
def skip_without_devtools_protocol(request: pytest.FixtureRequest, browser_name: str) -> None:
    if browser_name != "chromium" and request.node.get_closest_marker("chromium_only"):
        pytest.skip("test requires the Chrome DevTools Protocol")


@pytest.fixture
def page(context: BrowserContext) -> Generator[Page, None, None]:
    page = context.new_page()
    page.set_default_timeout(5000)
    page.set_default_navigation_timeout(5000)
    yield page
    page.close()
