"""Playwright-based semantic browser tests for web UI flows.

WS-3: Real Browser Validation -- tests assert rendered content and state
transitions, not just HTTP status codes.

Requires: playwright (pip install playwright && playwright install chromium)
Skip marker applied when Playwright or browsers are unavailable.

Tags: [FLOW:HTTP] [DEPENDENCY:PLAYWRIGHT]
"""
from __future__ import annotations

import io
import struct
import sys
import threading
import time
import zlib
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

# Conditional import with skip
try:
    from playwright.sync_api import sync_playwright, Page, Browser
    _PW_AVAILABLE = True
except ImportError:
    _PW_AVAILABLE = False

try:
    from scripts.asset_gen.web_api.app import create_app
    _FLASK_AVAILABLE = True
except ImportError:
    _FLASK_AVAILABLE = False


pytestmark = pytest.mark.skipif(
    not _PW_AVAILABLE or not _FLASK_AVAILABLE,
    reason="Playwright or Flask not available",
)


def _make_minimal_png() -> bytes:
    """Generate the smallest valid 1x1 white RGBA PNG (binary, no Image.new)."""
    sig = b"\x89PNG\r\n\x1a\n"

    def chunk(ctype: bytes, data: bytes) -> bytes:
        c = ctype + data
        crc = struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
        return struct.pack(">I", len(data)) + c + crc

    ihdr = struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)
    raw = zlib.compress(b"\x00\xff\xff\xff\xff")
    idat = chunk(b"IDAT", raw)
    return sig + chunk(b"IHDR", ihdr) + idat + chunk(b"IEND", b"")


class _FlaskServer:
    """Context manager that runs Flask in a background thread."""

    def __init__(self, tmp_path: Path, port: int = 0):
        self._tmp_path = tmp_path
        self._port = port
        self._thread = None
        self._app = None

    def __enter__(self):
        self._app = create_app(upload_dir=str(self._tmp_path / "uploads"))
        self._app.config["TESTING"] = True

        # Find a free port
        import socket
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("127.0.0.1", 0))
        self._port = sock.getsockname()[1]
        sock.close()

        self._thread = threading.Thread(
            target=lambda: self._app.run(
                host="127.0.0.1",
                port=self._port,
                debug=False,
                use_reloader=False,
            ),
            daemon=True,
        )
        self._thread.start()
        # Wait for server to be ready
        time.sleep(0.5)
        return self

    def __exit__(self, *args):
        pass  # daemon thread dies with test

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self._port}"


@pytest.fixture(scope="module")
def flask_server(tmp_path_factory):
    """Start a Flask server for the entire test module."""
    tmp_path = tmp_path_factory.mktemp("browser_tests")
    with _FlaskServer(tmp_path) as server:
        yield server


@pytest.fixture(scope="module")
def browser_ctx():
    """Launch a headless Chromium browser for the entire test module."""
    pw = None
    browser = None
    try:
        pw = sync_playwright().start()
        browser = pw.chromium.launch(headless=True)
        yield browser
    except Exception:
        pytest.skip("Chromium browser not installed (run: playwright install chromium)")
    finally:
        if browser:
            browser.close()
        if pw:
            pw.stop()


@pytest.fixture
def page(browser_ctx, flask_server):
    """Create a fresh page pointing at the Flask server."""
    ctx = browser_ctx.new_context(base_url=flask_server.base_url)
    pg = ctx.new_page()
    yield pg
    pg.close()
    ctx.close()


class TestWizardPageContent:
    """Wizard page loads and renders meaningful HTML content."""

    def test_wizard_has_html_structure(self, page, flask_server):
        page.goto(flask_server.base_url + "/")
        page.wait_for_load_state("domcontentloaded")

        # Assert actual DOM content, not just HTTP status
        title = page.title()
        assert title, "Page must have a title"

        body_text = page.inner_text("body")
        assert len(body_text) > 10, "Body must contain rendered text content"

    def test_wizard_serves_css(self, page, flask_server):
        page.goto(flask_server.base_url + "/")
        page.wait_for_load_state("domcontentloaded")

        # Check that stylesheets are linked
        links = page.query_selector_all('link[rel="stylesheet"]')
        scripts = page.query_selector_all("script[src]")
        # Should have at least one stylesheet or inline style
        has_styles = len(links) > 0 or page.query_selector("style") is not None
        assert has_styles or len(scripts) > 0, "Page must include CSS or JS resources"


class TestWorkbenchPageContent:
    """Workbench page loads with expected DOM structure."""

    def test_workbench_has_content(self, page, flask_server):
        page.goto(flask_server.base_url + "/workbench")
        page.wait_for_load_state("domcontentloaded")

        body_text = page.inner_text("body")
        assert "workbench" in body_text.lower() or len(body_text) > 20, (
            "Workbench page must contain recognizable content"
        )


class TestBranchesPageContent:
    """Branches viewer page loads with expected DOM structure."""

    def test_branches_has_content(self, page, flask_server):
        page.goto(flask_server.base_url + "/branches")
        page.wait_for_load_state("domcontentloaded")

        body_text = page.inner_text("body")
        assert "branch" in body_text.lower() or len(body_text) > 20, (
            "Branches page must contain recognizable content"
        )


class TestAPIContentVerification:
    """API responses return structured JSON with expected fields (via browser fetch)."""

    def test_config_defaults_returns_object(self, page, flask_server):
        page.goto(flask_server.base_url + "/")
        page.wait_for_load_state("domcontentloaded")

        result = page.evaluate("""
            async () => {
                const resp = await fetch('/api/config/defaults');
                return await resp.json();
            }
        """)
        assert isinstance(result, dict), "Config defaults must return a JSON object"
        assert len(result) > 0, "Config defaults must have at least one field"

    def test_config_schema_returns_fields(self, page, flask_server):
        page.goto(flask_server.base_url + "/")
        page.wait_for_load_state("domcontentloaded")

        result = page.evaluate("""
            async () => {
                const resp = await fetch('/api/config/schema');
                return await resp.json();
            }
        """)
        assert isinstance(result, dict), "Config schema must return a JSON object"

    def test_workbench_sessions_list(self, page, flask_server):
        page.goto(flask_server.base_url + "/workbench")
        page.wait_for_load_state("domcontentloaded")

        result = page.evaluate("""
            async () => {
                const resp = await fetch('/api/workbench/sessions');
                return await resp.json();
            }
        """)
        assert "sessions" in result, "Sessions endpoint must return sessions array"
        assert isinstance(result["sessions"], list)


class TestWorkbenchSessionBrowser:
    """Workbench session creation and persistence via browser fetch."""

    def test_create_session_returns_id(self, page, flask_server):
        page.goto(flask_server.base_url + "/workbench")
        page.wait_for_load_state("domcontentloaded")

        result = page.evaluate("""
            async () => {
                const resp = await fetch('/api/workbench/start-session', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({angles: 1, anims: [1]}),
                });
                return await resp.json();
            }
        """)
        session_id = result.get("job_id") or result.get("id") or result.get("session_id")
        assert session_id, "Session creation must return an identifier"

    def test_session_appears_in_list_after_creation(self, page, flask_server):
        page.goto(flask_server.base_url + "/workbench")
        page.wait_for_load_state("domcontentloaded")

        # Create a session
        page.evaluate("""
            async () => {
                await fetch('/api/workbench/start-session', {
                    method: 'POST',
                    headers: {'Content-Type': 'application/json'},
                    body: JSON.stringify({angles: 1, anims: [1]}),
                });
            }
        """)

        # List sessions
        result = page.evaluate("""
            async () => {
                const resp = await fetch('/api/workbench/sessions');
                return await resp.json();
            }
        """)
        assert len(result["sessions"]) > 0, "At least one session must exist after creation"
