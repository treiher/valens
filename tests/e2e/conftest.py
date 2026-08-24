from __future__ import annotations

import logging
from collections.abc import Generator

import pytest
from playwright.sync_api import BrowserContext, Page, Response

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)


@pytest.fixture(autouse=True)
def skip_without_devtools_protocol(request: pytest.FixtureRequest, browser_name: str) -> None:
    if browser_name != "chromium" and request.node.get_closest_marker("chromium_only"):
        pytest.skip("test requires the Chrome DevTools Protocol")


@pytest.fixture(autouse=True)
def skip_incompatible_webkit(request: pytest.FixtureRequest, browser_name: str) -> None:
    if browser_name == "webkit" and request.node.get_closest_marker("webkit_incompatible"):
        pytest.skip("test requires behavior which WebKit does not provide")


@pytest.fixture
def page(context: BrowserContext) -> Generator[Page, None, None]:
    page = context.new_page()
    page.set_default_timeout(5000)
    page.set_default_navigation_timeout(5000)
    page.on("console", lambda message: logger.info("console [%s] %s", message.type, message.text))
    page.on("pageerror", lambda error: logger.info("page error: %s", error))
    page.on(
        "requestfailed",
        lambda request: logger.info("request failed: %s %s", request.url, request.failure),
    )
    page.on("response", log_html_response)
    yield page
    page.close()


def log_html_response(response: Response) -> None:
    """Log resources answered with HTML, which the backend serves for unknown paths."""

    if response.request.resource_type == "document":
        return

    # `headers` is used instead of `header_value`, as the latter queries the browser and fails when
    # the page is closed while responses are still in flight.
    if response.headers.get("content-type", "").startswith("text/html"):
        logger.info("HTML response for %s", response.url)
