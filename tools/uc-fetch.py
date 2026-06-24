#!/usr/bin/env python3
from __future__ import annotations

import asyncio
import os
import sys
from http.cookies import SimpleCookie

CHALLENGE_MARKERS = (
    "cf-browser-verification",
    "cf_clearance",
    "Checking your browser",
    "Just a moment",
    "challenge-platform",
    "cf-error-code",
)


def cookie_header_to_playwright(header: str, url: str) -> list[dict[str, str]]:
    jar = SimpleCookie()
    jar.load(header or "")
    domain = url.split("/", 3)[2]
    return [
        {"name": morsel.key, "value": morsel.value, "domain": domain, "path": "/"}
        for morsel in jar.values()
    ]


def looks_challenged(text: str) -> bool:
    return any(marker in text for marker in CHALLENGE_MARKERS)


def try_curl_cffi(url: str, cookie: str) -> str | None:
    try:
        from curl_cffi import requests
    except Exception:
        return None
    headers = {
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
        "Cookie": cookie,
    }
    response = requests.get(url, headers=headers, impersonate="chrome146", timeout=30)
    text = response.text
    if response.status_code < 400 and not looks_challenged(text):
        return text
    return None


async def try_camoufox(url: str, cookie: str) -> str:
    os.environ.setdefault("PLAYWRIGHT_HOST_PLATFORM_OVERRIDE", "ubuntu24.04-x64")
    from camoufox.async_api import AsyncCamoufox

    async with AsyncCamoufox(headless=True, geoip=True, block_webrtc=True) as browser:
        page = await browser.new_page()
        cookies = cookie_header_to_playwright(cookie, url)
        if cookies:
            await browser.contexts[0].add_cookies(cookies)
        await page.goto(url, wait_until="networkidle", timeout=60_000)
        await page.wait_for_timeout(2500)
        html = await page.content()
        await page.close()
        if looks_challenged(html):
            raise RuntimeError("Cloudflare challenge still present after Camoufox fetch")
        return html


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: uc-fetch.py URL", file=sys.stderr)
        return 2
    url = sys.argv[1]
    cookie = os.environ.get("FORUM_COOKIE", "")

    html = try_curl_cffi(url, cookie)
    if html is not None:
        print(html)
        return 0

    print(asyncio.run(try_camoufox(url, cookie)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
