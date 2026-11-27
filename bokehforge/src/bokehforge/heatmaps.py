"""Serve a rendered image-interest gallery for local comparison."""

import argparse
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from io import BytesIO
import json
from os import fstat
from pathlib import Path
import re
from shutil import copyfileobj
from socket import AF_INET6


def load_gallery(directory: Path) -> bytes:
    manifest = directory / "interest-data.js"
    declaration = manifest.read_text(encoding="utf-8").strip()
    prefix = "const gallery = "
    if not declaration.startswith(prefix) or not declaration.endswith(";"):
        raise ValueError(f"{manifest} must contain a generated gallery declaration")
    gallery = json.loads(declaration[len(prefix):-1])
    if not isinstance(gallery, dict) or not gallery.get("scenes") or not isinstance(gallery.get("models"), dict):
        raise ValueError(f"{manifest} must contain gallery scenes and model descriptions")
    return json.dumps(gallery, allow_nan=False).encode("utf-8")


class GalleryHandler(BaseHTTPRequestHandler):
    def __init__(self, *args, image_directory: Path, gallery: bytes, **kwargs):
        self.image_directory = image_directory
        self.gallery = gallery
        super().__init__(*args, **kwargs)

    def do_GET(self):
        self.serve_asset(include_body=True)

    def do_HEAD(self):
        self.serve_asset(include_body=False)

    def serve_asset(self, *, include_body: bool):
        path = self.path.partition("?")[0]
        if path in {"/", "/index.html", "/interest.html", "/classic.html"}:
            content = Path(__file__).with_name("heatmaps.html").read_bytes()
            asset, size, media_type = BytesIO(content), len(content), "text/html; charset=utf-8"
        elif path == "/gallery.json":
            asset, size, media_type = BytesIO(self.gallery), len(self.gallery), "application/json"
        elif re.fullmatch(r"/interest-v1/[A-Za-z0-9_-]+\.png", path):
            image = (self.image_directory / path.rsplit("/", 1)[-1]).resolve()
            if not image.is_relative_to(self.image_directory) or not image.is_file():
                self.send_error(404)
                return
            asset = image.open("rb")
            size, media_type = fstat(asset.fileno()).st_size, "image/png"
        else:
            self.send_error(404)
            return

        with asset:
            self.send_response(200)
            self.send_header("Content-Type", media_type)
            self.send_header("Content-Length", str(size))
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            if include_body:
                try:
                    copyfileobj(asset, self.wfile)
                except (BrokenPipeError, ConnectionResetError):
                    pass


class IPv6GalleryServer(ThreadingHTTPServer):
    address_family = AF_INET6


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="bokehforge heatmaps", description="Compare rendered image-interest heatmaps in a browser.",
    )
    parser.add_argument("gallery", type=Path, help="Directory containing interest-data.js and interest-v1/")
    connection = parser.add_argument_group("Connection")
    connection.add_argument("--host", default="127.0.0.1", help="Bind address (default: %(default)s)")
    connection.add_argument("--port", type=int, default=8765, help="TCP port (default: %(default)s)")
    args = parser.parse_args(argv)
    if not 0 <= args.port <= 65535:
        parser.error("--port must be between 0 and 65535")
    directory = args.gallery.resolve()
    gallery = load_gallery(directory)
    image_directory = directory / "interest-v1"
    if not image_directory.is_dir():
        raise ValueError(f"Gallery image directory does not exist: {image_directory}")

    handler = partial(GalleryHandler, image_directory=image_directory, gallery=gallery)
    server_class = IPv6GalleryServer if ":" in args.host else ThreadingHTTPServer
    with server_class((args.host, args.port), handler) as server:
        display_host = f"[{args.host}]" if ":" in args.host else args.host
        print(f"BokehForge heatmaps at http://{display_host}:{server.server_port}/", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
    return 0
