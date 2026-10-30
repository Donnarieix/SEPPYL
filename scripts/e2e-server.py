#!/usr/bin/env python3
"""Local-only static SPA server + minimal same-origin API proxy for E2E.
Never binds publicly; emulates the existing production Nginx /api rules.
"""
import argparse
import http.client
import http.server
import pathlib
import urllib.parse

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, directory=None, **kwargs):
        super().__init__(*args, directory=directory, **kwargs)

    def do_GET(self):
        self.dispatch()
    def do_POST(self):
        self.dispatch()
    def do_PUT(self):
        self.dispatch()
    def do_DELETE(self):
        self.dispatch()

    def dispatch(self):
        parsed = urllib.parse.urlsplit(self.path)
        if parsed.path == "/api/register":
            self.send_error(403, "Registration disabled by reverse proxy")
            return
        if parsed.path.startswith("/api/"):
            target = parsed.path[4:] + (("?" + parsed.query) if parsed.query else "")
            length = int(self.headers.get("Content-Length") or "0")
            if length > 4 * 1024 * 1024:
                self.send_error(413)
                return
            body = self.rfile.read(length) if length else None
            conn = http.client.HTTPConnection("127.0.0.1", 3101, timeout=15)
            headers = {k: v for k, v in self.headers.items()
                       if k.lower() in ("authorization", "content-type")}
            try:
                conn.request(self.command, target, body=body, headers=headers)
                response = conn.getresponse()
                payload = response.read()
                self.send_response(response.status)
                for h in ("Content-Type", "Cache-Control"):
                    val = response.getheader(h)
                    if val:
                        self.send_header(h, val)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            except OSError as exc:
                self.send_error(502, "E2E API unavailable: " + str(exc))
            finally:
                conn.close()
            return
        if self.command != "GET":
            self.send_error(405)
            return
        if parsed.path == "/":
            self.path = "/index.html"
        else:
            requested = pathlib.PurePosixPath(urllib.parse.unquote(parsed.path))
            relative = pathlib.PurePosixPath(str(requested).lstrip("/"))
            resolved = pathlib.Path(self.directory, str(relative)).resolve()
            root = pathlib.Path(self.directory).resolve()
            if not resolved.is_relative_to(root):
                self.send_error(403)
                return
            if not resolved.is_file():
                # Browser routes need SPA fallback, but missing static assets
                # must remain 404 so the E2E console test detects them.
                if "." in requested.name or parsed.path.startswith("/assets/"):
                    self.send_error(404)
                    return
                self.path = "/index.html"
            else:
                self.path = parsed.path
        super().do_GET()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    args = parser.parse_args()
    root = pathlib.Path(args.root).resolve()
    assert (root / "index.html").is_file(), "Dioxus release index.html is missing"
    def factory(*args, **kwargs):
        return Handler(*args, directory=str(root), **kwargs)
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 4173), factory)
    print("E2E frontend and API proxy listening on 127.0.0.1:4173", flush=True)
    server.serve_forever()
