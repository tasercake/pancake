"""Browse and generate heatmaps for BokehForge datasets."""

import argparse
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.util import find_spec
from ipaddress import ip_address
import json
from pathlib import Path
import re
from socket import AF_INET6
from threading import Lock, Thread
from urllib.parse import parse_qs, urlsplit

from ._device import cuda_available
from .data._config import DEFAULT_CONFIG, data_root, read_config
from .heatmap_cache import HeatmapCache, PROFILES
from .review.events import Events


def _inspection_available():
    packages = ("torchvision", "transformers", "timm", "pandas", "ftfy", "einops", "cv2")
    return all(find_spec(package) for package in packages) and cuda_available()


class Gallery:
    def __init__(self, cache):
        self.cache = cache
        self.events = Events()
        self.lock = Lock()
        self.cached = {
            profile: {dataset: gallery for dataset in cache.sources if (gallery := cache.gallery(dataset, profile))}
            for profile in PROFILES
        }
        self.generators = {"baseline"}
        if _inspection_available():
            self.generators.add("inspection")
        self.job = {"status": "idle"}
        self.publish()

    def publish(self):
        self.events.publish("heatmaps", lambda: {
            "datasets": [
                {"id": dataset, "name": source["name"],
                 "cached": [profile for profile, ready in self.cached.items() if dataset in ready]}
                for dataset, source in self.cache.sources.items()
            ],
            "profiles": [
                {"id": profile, "name": name, "can_generate": profile in self.generators}
                for profile, name in PROFILES.items() if profile in self.generators or self.cached[profile]
            ],
            "job": self.job,
        })

    def selection(self, profile, datasets):
        if not isinstance(profile, str) or profile not in PROFILES:
            raise ValueError("Choose a heatmap model group")
        if (not isinstance(datasets, list) or not datasets
                or any(not isinstance(dataset, str) or dataset not in self.cache.sources for dataset in datasets)):
            raise ValueError("Choose at least one BokehForge dataset")
        return profile, list(dict.fromkeys(datasets))

    def gallery(self, profile, datasets):
        profile, datasets = self.selection(profile, datasets)
        with self.lock:
            cached = [self.cached[profile][dataset] for dataset in datasets if dataset in self.cached[profile]]
        return {"scenes": [scene for gallery in cached for scene in gallery["scenes"]],
                "models": {key: value for gallery in cached for key, value in gallery["models"].items()}}

    def start(self, profile, datasets):
        profile, datasets = self.selection(profile, datasets)
        if profile not in self.generators:
            raise ValueError("Generating Inspection maps requires the inspection dependencies and CUDA")
        with self.lock:
            if self.job["status"] == "running":
                return False
            self.job = {"status": "running", "profile": profile, "message": "Preparing heatmap generation"}
            self.publish()
            Thread(target=self.generate, args=(profile, datasets), name="bokehforge-heatmaps", daemon=True).start()
        return True

    def generate(self, profile, datasets):
        def report(message, completed=0, total=None, unit="scenes"):
            with self.lock:
                self.job.update(message=message, completed=completed, total=total, unit=unit)
                self.publish()

        def ready(dataset, gallery):
            with self.lock:
                self.cached[profile][dataset] = gallery
                self.publish()

        status, message = "complete", "Heatmaps ready"
        try:
            self.cache.generate(datasets, profile, report, ready)
        except Exception as exc:
            status, message = "failed", str(exc) or type(exc).__name__
        finally:
            with self.lock:
                self.job.update(status=status, message=message)
                self.publish()


class GalleryHandler(BaseHTTPRequestHandler):
    timeout = 10

    def __init__(self, *args, app, host, **kwargs):
        self.app = app
        self.host = host
        super().__init__(*args, **kwargs)

    def authorized(self):
        destination = self.connection.getsockname()[0]
        names = {self.host.lower(), destination}
        if ip_address(destination).is_loopback:
            names.add("localhost")
        authority = self.headers.get("Host", "").lower()
        allowed = {f"{'[' + name + ']' if ':' in name else name}:{self.server.server_port}" for name in names}
        if self.server.server_port == 80:
            allowed |= {name.removesuffix(":80") for name in allowed}
        origin = self.headers.get("Origin")
        return authority in allowed and (origin is None or origin == f"http://{authority}")

    def do_GET(self):
        self.get()

    def do_HEAD(self):
        self.get()

    def get(self):
        if not self.authorized():
            self.send_error(403, "Use the local heatmap viewer address")
            return
        try:
            parts = urlsplit(self.path)
            asset = re.fullmatch(r"/images/([a-z]+)/([A-Za-z0-9_-]+)/([a-z0-9-]+)\.png", parts.path)
            if parts.path in {"/", "/index.html", "/interest.html", "/classic.html"}:
                self.send(200, "text/html; charset=utf-8", Path(__file__).with_name("heatmaps.html").read_bytes())
            elif parts.path == "/api/events" and self.command == "GET":
                self.send_events()
            elif parts.path == "/api/gallery":
                query = parse_qs(parts.query)
                self.send_json(200, self.app.gallery(query.get("profile", [None])[0], query.get("dataset", [])))
            elif asset and asset[1] in PROFILES and asset[2] in self.app.cache.sources:
                profile, dataset, name = asset.groups()
                image = self.app.cache.image(dataset, profile, name)
                if image is None:
                    self.send_error(404)
                else:
                    self.send(200, "image/png", image)
            else:
                self.send_error(404)
        except ValueError as exc:
            self.send_json(400, {"error": str(exc)})

    def do_POST(self):
        if not self.authorized():
            self.send_error(403, "Use the local heatmap viewer address")
            return
        if self.path != "/api/generate":
            self.send_error(404)
            return
        if self.headers.get_content_type() != "application/json":
            self.send_error(415)
            return
        try:
            size = int(self.headers.get("Content-Length", 0))
            if not 0 < size <= 4096:
                raise ValueError("Invalid generation request size")
            data = json.loads(self.rfile.read(size))
            if not isinstance(data, dict):
                raise ValueError("Invalid generation request")
            if self.app.start(data.get("profile"), data.get("datasets")):
                self.send_json(202, {"status": "running"})
            else:
                self.send_json(409, {"error": "Heatmap generation is already running"})
        except ValueError as exc:
            self.send_json(400, {"error": str(exc)})

    def send_events(self):
        self.close_connection = True
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream; charset=utf-8")
        self.send_header("Cache-Control", "no-store, no-transform")
        self.send_header("X-Accel-Buffering", "no")
        self.end_headers()
        self.app.events.stream(self.connection, self.wfile.write)

    def send_json(self, status, body):
        self.send(status, "application/json", json.dumps(body).encode())

    def send(self, status, media_type, body):
        self.send_response(status)
        self.send_header("Content-Type", media_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store" if media_type == "application/json" else "no-cache")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionResetError, TimeoutError):
            pass


class IPv6GalleryServer(ThreadingHTTPServer):
    address_family = AF_INET6


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="bokehforge heatmaps", description="Generate and compare heatmaps for BokehForge datasets.",
    )
    connection = parser.add_argument_group("Connection")
    connection.add_argument("--host", default="127.0.0.1", help="Bind address (default: %(default)s)")
    connection.add_argument("--port", type=int, default=8765, help="TCP port (default: %(default)s)")
    args = parser.parse_args(argv)
    if not 0 <= args.port <= 65535:
        parser.error("--port must be between 0 and 65535")
    sources = read_config(DEFAULT_CONFIG)
    app = Gallery(HeatmapCache(data_root(defaults=sources.get("defaults", {})), sources["datasets"]))
    handler = partial(GalleryHandler, app=app, host=args.host)
    server_class = IPv6GalleryServer if ":" in args.host else ThreadingHTTPServer
    with server_class((args.host, args.port), handler) as server:
        display_host = f"[{args.host}]" if ":" in args.host else args.host
        print(f"BokehForge heatmaps at http://{display_host}:{server.server_port}/", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            app.events.close()
    return 0
