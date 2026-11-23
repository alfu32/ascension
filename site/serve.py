#!/usr/bin/env python3
"""The public site's web server (task 135): serves site/dist as uploaded to the server's home.

Runs as the server user's systemd service `ascension-site.service` on 127.0.0.1 only; nginx
proxies every path except the game server's /ws and /status to it. Python standard library only.

- The root is a symlink (`current`) that site/deploy.sh switches atomically; it is resolved on
  every request, so a new upload is live at once without a restart.
- Pages are revalidated on every visit (`no-cache` with Last-Modified); pictures, sounds, music,
  styles and scripts carry a content hash in their names and are kept for a year.
- `/story` serves `story.html`; byte ranges are answered (206) so audio can seek.
- Security headers on every answer: everything from this site, nothing from anywhere else.
"""

import argparse
import email.utils
import mimetypes
import os
import posixpath
import urllib.parse
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HASHED = ("/img/", "/music/", "/sounds/", "/static/")
TYPES = {
    ".html": "text/html; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".js": "application/javascript; charset=utf-8",
    ".json": "application/json",
    ".svg": "image/svg+xml",
    ".webp": "image/webp",
    ".jpg": "image/jpeg",
    ".png": "image/png",
    ".ogg": "audio/ogg",
    ".ico": "image/x-icon",
    ".txt": "text/plain; charset=utf-8",
}
CSP = (
    "default-src 'self'; img-src 'self'; media-src 'self'; script-src 'self'; "
    "style-src 'self'; connect-src 'self'; font-src 'self'; object-src 'none'; "
    "base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
)
CHUNK = 256 * 1024


class Site(BaseHTTPRequestHandler):
    server_version = "ascension-site"
    sys_version = ""
    root = "."

    def log_message(self, format, *args):  # noqa: A002 - the base class's name
        pass  # nginx keeps the access log

    def do_HEAD(self):
        self.serve(head=True)

    def do_GET(self):
        self.serve(head=False)

    def resolve(self):
        """The file for the request path inside the (resolved) root, or None."""
        path = urllib.parse.unquote(urllib.parse.urlsplit(self.path).path)
        path = posixpath.normpath(path)
        if not path.startswith("/") or "\x00" in path:
            return None, path
        root = os.path.realpath(self.root)
        rel = path.lstrip("/")
        candidates = [rel or "index.html"]
        if rel and not os.path.splitext(rel)[1]:
            candidates.append(rel + ".html")
            candidates.append(posixpath.join(rel, "index.html"))
        for candidate in candidates:
            full = os.path.realpath(os.path.join(root, candidate))
            if full != root and not full.startswith(root + os.sep):
                return None, path
            if os.path.isfile(full):
                return full, path
        return None, path

    def headers_common(self, path, ctype):
        if path.startswith(HASHED):
            self.send_header("Cache-Control", "public, max-age=31536000, immutable")
        else:
            self.send_header("Cache-Control", "no-cache")
        if ctype.startswith("text/html"):
            self.send_header("Content-Security-Policy", CSP)
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Referrer-Policy", "no-referrer")

    def serve(self, head):
        full, path = self.resolve()
        if full is None:
            self.not_found(head)
            return
        stat = os.stat(full)
        ext = os.path.splitext(full)[1].lower()
        ctype = TYPES.get(ext) or mimetypes.guess_type(full)[0] or "application/octet-stream"
        modified = email.utils.formatdate(stat.st_mtime, usegmt=True)
        since = self.headers.get("If-Modified-Since")
        if since and self.headers.get("Range") is None:
            try:
                if email.utils.parsedate_to_datetime(since).timestamp() >= int(stat.st_mtime):
                    self.send_response(HTTPStatus.NOT_MODIFIED)
                    self.headers_common(path, ctype)
                    self.end_headers()
                    return
            except (TypeError, ValueError):
                pass
        size = stat.st_size
        start, end = 0, size - 1
        status = HTTPStatus.OK
        wanted = self.headers.get("Range")
        if wanted and wanted.startswith("bytes=") and "," not in wanted and size > 0:
            first, _, last = wanted[6:].partition("-")
            try:
                if first:
                    start = int(first)
                    end = min(int(last), size - 1) if last else size - 1
                else:
                    start = max(size - int(last), 0)
                if start > end or start >= size:
                    raise ValueError
                status = HTTPStatus.PARTIAL_CONTENT
            except ValueError:
                self.send_response(HTTPStatus.REQUESTED_RANGE_NOT_SATISFIABLE)
                self.send_header("Content-Range", f"bytes */{size}")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(end - start + 1))
        self.send_header("Last-Modified", modified)
        self.send_header("Accept-Ranges", "bytes")
        if status == HTTPStatus.PARTIAL_CONTENT:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.headers_common(path, ctype)
        self.end_headers()
        if head:
            return
        with open(full, "rb") as f:
            f.seek(start)
            left = end - start + 1
            while left > 0:
                block = f.read(min(CHUNK, left))
                if not block:
                    break
                try:
                    self.wfile.write(block)
                except (BrokenPipeError, ConnectionResetError):
                    return
                left -= len(block)

    def not_found(self, head):
        page = os.path.join(os.path.realpath(self.root), "404.html")
        body = b"Not found\n"
        ctype = "text/plain; charset=utf-8"
        if os.path.isfile(page):
            with open(page, "rb") as f:
                body = f.read()
            ctype = "text/html; charset=utf-8"
        self.send_response(HTTPStatus.NOT_FOUND)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.headers_common("/404", ctype)
        self.end_headers()
        if not head:
            self.wfile.write(body)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", required=True, help="the site folder (the `current` symlink)")
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=18090)
    args = parser.parse_args()
    Site.root = args.root
    server = ThreadingHTTPServer((args.bind, args.port), Site)
    server.daemon_threads = True
    server.serve_forever()


if __name__ == "__main__":
    main()
