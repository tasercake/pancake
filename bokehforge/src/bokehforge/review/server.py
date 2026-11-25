"""Serve the BokehForge rating app over HTTP."""

import argparse
from hashlib import sha256
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from ipaddress import ip_address
import json
from pathlib import Path
from secrets import token_hex
from socket import SO_KEEPALIVE, SOL_SOCKET, gethostname
from threading import Lock, Thread
from urllib.parse import parse_qs, urlsplit

from .. import SceneCatalog
from ..data._canonical import RETIRED_LAYOUTS
from ..data._config import DEFAULT_CONFIG, read_config
from ..data._files import safe_path
from .downloads import Downloads
from .events import Events
from .images import (
    DecodedFrames, PREVIEW_VERSION, PreviewCache, ReviewFrameDecoder, SIZES, output_pixels, preview, preview_mime,
)
from .ratings import Ratings
from .runs import Runs, lock_store
from .stacker_setup import REPO
from .stackers import configured_stackers

STATIC = Path(__file__).with_name("static")
ASSETS = {"/": ("index.html", "text/html; charset=utf-8"),
          "/app.js": ("app.js", "text/javascript; charset=utf-8"),
          "/rail.js": ("rail.js", "text/javascript; charset=utf-8"),
          "/style.css": ("style.css", "text/css; charset=utf-8")}
ALL_INTERFACES = "0.0.0.0"


def local_sources(args):
    policies = {source["id"]: source for source in read_config(args.config or DEFAULT_CONFIG)["datasets"]
                if source.get("layout") not in RETIRED_LAYOUTS}
    selected = set(args.dataset or policies)
    if selected - policies.keys():
        raise ValueError(f"Unknown dataset IDs: {sorted(selected - policies.keys())}")
    locked = (json.loads(args.lock.read_text())["datasets"]
              if args.lock is not None and args.lock.is_file() else [])
    by_id = {source["id"]: source for source in locked}
    present, missing = {}, {}
    for dataset_id in sorted(selected):
        policy = policies[dataset_id]
        if policy.get("provider") == "disabled":
            continue
        raw = safe_path(args.root / "raw", dataset_id)
        path = raw / "download-manifest.json"
        source = by_id.get(dataset_id, {}) if args.lock is not None else (
            json.loads(path.read_text()) if path.is_file() else {})
        if source.get("status") == "disabled":
            continue
        files = source.get("files", [])
        if source.get("id") == dataset_id and source.get("status") == "available" and files and all(
            (asset := safe_path(raw, item["path"])).is_file()
            and (item.get("size") is None or asset.stat().st_size == item["size"])
            for item in files
        ):
            present[dataset_id] = source.get("name") or policy.get("name") or dataset_id
        else:
            missing[dataset_id] = policy.get("name") or dataset_id
    return present, missing


class CatalogPending(RuntimeError):
    pass


class Review:
    def __init__(self, args):
        self.args = args
        self.catalog = None
        self.catalog_status = {"status": "indexing", "root": str(args.root), "revision": 0, "missing": {}}
        self.scenes = []
        self.catalog_lock = Lock()
        self.indexing = False
        self.refresh_requested = False
        self.events = Events()
        self.runs = Runs(args.store, configured_stackers(args), self.publish_runs)
        self.ratings = Ratings(args.store)
        self.image_version = token_hex(8)
        self.previews = PreviewCache(args.store)
        self.decoded = DecodedFrames()
        self.frame_decoder = ReviewFrameDecoder()
        self.downloads = Downloads(args.root, args.config or DEFAULT_CONFIG, args.lock,
                                   self.refresh_catalog, self.publish_download)
        self.publish_runs()
        self.events.publish("catalog", self.catalog_data)
        self.refresh_catalog()

    def catalog_data(self):
        return {**self.catalog_status, "stackers": self.runs.tooling(), "image_version": self.image_version}

    def publish_runs(self):
        self.events.publish("runs", self.runs.list)

    def publish_download(self):
        self.events.publish("download", self.download_status)

    def refresh_catalog(self):
        with self.catalog_lock:
            self.refresh_requested = True
            if not self.indexing:
                self.indexing = True
                Thread(target=self._index, daemon=True, name="bokehforge-catalog").start()
        self.publish_download()

    def _index(self):
        while True:
            with self.catalog_lock:
                self.refresh_requested = False
            self._load()
            with self.catalog_lock:
                pending = self.refresh_requested
                self.indexing = pending
            self.events.publish("catalog", self.catalog_data)
            self.publish_download()
            if not pending:
                return

    def _load(self):
        args = self.args
        revision = self.catalog_status["revision"] + 1
        summary = {"root": str(args.root), "revision": revision, "missing": {}}
        try:
            present, missing = local_sources(args)
            summary["missing"] = missing
            if not present:
                self.catalog = None
                self.scenes = []
                self.catalog_status = {**summary, "status": "missing" if missing else "empty"}
                return
            catalog = SceneCatalog(args.root, datasets=tuple(present), config=args.config, lock=args.lock)
            scenes = [catalog.scene(scene_id) for scene_id in catalog.scene_ids]
            entries = [
                {"id": scene.id, "name": scene.metadata["source_scene_id"],
                 "dataset": scene.metadata["dataset_id"], "frames": len(scene.frame_ids),
                 "first_frame": scene.frame_ids[0],
                 "first_reference": next(iter(scene.reference_ids), None),
                 "preview_frame": scene.frame_ids[len(scene.frame_ids) // 2]}
                for scene in scenes
            ]
            self.catalog = catalog
            self.scenes = entries
            self.catalog_status = {**summary, "status": "ready" if entries else "empty",
                                   "scenes": entries, "datasets": present}
        except Exception as exc:
            if self.catalog is None:
                self.catalog_status = {**summary, "status": "error", "error": str(exc)}
            else:
                self.catalog_status = {**self.catalog_status, **summary, "error": str(exc)}

    def download_status(self):
        catalog = self.catalog_status
        return {**self.downloads.snapshot(), "indexing": self.indexing,
                "catalog_revision": catalog["revision"], "missing": catalog["missing"],
                "catalog_error": catalog.get("error")}

    def start_download(self):
        if self.indexing:
            raise RuntimeError("Wait for Review to check the local datasets")
        self.downloads.start(tuple(self.catalog_status["missing"]))
        return self.download_status()

    def require_catalog(self):
        if self.catalog is None:
            raise CatalogPending(self.catalog_status.get("error", "Indexing local datasets"))
        return self.catalog

    def scene_details(self, scene_id, run_id):
        catalog = self.require_catalog()
        scene = catalog.scene(scene_id)
        ratings = self.ratings.get(run_id, scene.id) if run_id else {}
        outputs = self.runs.scene_results(run_id, catalog, scene)
        for result in outputs:
            result["rating"] = ratings.get(result["stacker"]) if result["status"] == "ready" else None
        return {"frames": [self.frame_info(catalog, frame_id) for frame_id in scene.frame_ids],
                "references": [self.frame_info(catalog, frame_id) for frame_id in scene.reference_ids],
                "reference_type": scene.metadata["reference_type"],
                "reference_semantics": scene.metadata["reference_semantics"],
                "outputs": outputs}

    @staticmethod
    def frame_info(catalog, frame_id):
        frame = catalog.frame(frame_id)
        return {"id": frame_id, "name": frame.source_path.split("/")[-1].split("::")[-1],
                "page": frame.page + 1, **frame.metadata}

    def artifact(self, query, key):
        catalog = self.require_catalog()
        scene = catalog.scene(query["scene"])
        run_id = query["run"]
        for row in self.runs.scene_results(run_id, catalog, scene):
            if row["stacker"] == query["stacker"] and key in row:
                return self.runs.directory(run_id) / row[key]
        raise KeyError(f"Evaluation {key} unavailable")

    def image_target(self, query):
        catalog = self.require_catalog()
        variant = query.get("size", "fit")
        if variant not in SIZES:
            raise ValueError("Unknown image size")
        if "frame" in query:
            frame_id = query["frame"]
            frame = catalog.frame(frame_id)
            recipe = catalog.scene(frame.scene_id).metadata["recipe_sha256"]
            identity = (frame.asset_sha256, frame.source_sha256, frame.page, recipe)
            def read():
                return self.frame_decoder.decode(frame).data
        else:
            path = self.artifact(query, "image")
            stat = path.stat()
            identity = (str(path), stat.st_ino, stat.st_mtime_ns, stat.st_ctime_ns, stat.st_size)
            def read():
                return output_pixels(path)
        return variant, identity, read

    def image(self, query):
        """Render an image for internal callers."""
        variant, identity, read = self.image_target(query)
        key = self.image_key(variant, identity)
        return preview_mime(variant), self.image_body(key, variant, identity, read)

    @staticmethod
    def image_key(variant, identity):
        parts = (PREVIEW_VERSION, variant, preview_mime(variant), identity)
        return sha256(json.dumps(parts).encode()).hexdigest()

    def image_body(self, key, variant, identity, read, background=False):
        return self.previews.render(
            key, lambda: preview(self.decoded.get(identity, read), variant),
            background=background or variant == "thumb", persistent=variant != "native",
        )

    def start(self, form):
        catalog = self.require_catalog()
        kind = form["scope"]
        if kind == "scene":
            scene = catalog.scene(form["scene"])
            scene_ids = [scene.id]
            label = scene.metadata["source_scene_id"]
        elif kind == "dataset":
            label = form["dataset"]
            scene_ids = [scene_id for scene_id in catalog.scene_ids
                         if catalog.scene(scene_id).metadata["dataset_id"] == label]
        elif kind == "all":
            label = "All datasets"
            scene_ids = catalog.scene_ids
        else:
            raise ValueError("Unknown evaluation scope")
        if not scene_ids:
            raise ValueError("No scenes found for this scope")
        stackers = form["stackers"]
        if not isinstance(stackers, list) or any(not isinstance(key, str) for key in stackers):
            raise ValueError("Stackers must be a list of supported IDs")
        options = form.get("options", {})
        if not isinstance(options, dict):
            raise ValueError("Stacker settings must be an object")
        return self.runs.start(catalog, scene_ids, stackers, {"type": kind, "label": label}, options)

    def rate(self, form):
        self.artifact(form, "image")
        return self.ratings.save(form["run"], form["scene"], form["stacker"], form["grade"],
                                 form.get("comment", ""))


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    timeout = 10
    disable_nagle_algorithm = True

    def do_GET(self):
        self.handle_request(self.get)

    def do_POST(self):
        self.close_connection = True
        self.handle_request(self.post)

    def handle_request(self, route):
        host = self.headers.get("Host", "").lower()
        port = self.server.server_port
        destination = self.connection.getsockname()[0]
        names = self.server.review_hosts | {destination}
        if ip_address(destination).is_loopback:
            names.add("localhost")
        # Trust the literal destination of this connection and configured names.
        # A hostname that merely resolves here could be used for DNS rebinding.
        if port == 80:
            host = host.removesuffix(":80")
        else:
            names = {f"{name}:{port}" for name in names}
        if host not in names:
            self.json(403, {"error": "Use this computer's hostname or IP address and the review port"})
            return
        origin = self.headers.get("Origin")
        if origin is not None and origin != f"http://{host}":
            self.json(403, {"error": "Cross-origin request refused"})
            return
        try:
            parts = urlsplit(self.path)
            route(parts.path, {key: values[-1] for key, values in parse_qs(parts.query).items()})
        except (BrokenPipeError, ConnectionResetError):
            pass
        except KeyError as exc:
            self.json(404, {"error": str(exc.args[0])})
        except CatalogPending as exc:
            self.json(503, {"error": str(exc)})
        except (ValueError, TypeError) as exc:
            self.json(400, {"error": str(exc)})
        except Exception as exc:
            self.log_error("%s", exc)
            self.json(500, {"error": str(exc)})

    def get(self, path, query):
        if path in self.server.assets:
            mime, body = self.server.assets[path]
            self.send(200, mime, body)
            return
        app = self.server.app
        if path == "/api/events":
            self.send_events(app)
        elif path == "/api/catalog":
            self.json(200, app.catalog_data())
        elif path == "/api/download":
            self.json(200, app.download_status())
        elif path == "/api/runs":
            self.json(200, app.runs.list())
        elif path == "/api/scene":
            self.json(200, app.scene_details(query["scene"], query.get("run")))
        elif path == "/api/image":
            self.send_image(app, query)
        elif path == "/api/log":
            log = app.artifact(query, "log")
            with log.open("rb") as handle:
                handle.seek(max(0, log.stat().st_size - 24000))
                body = handle.read().decode("utf-8", errors="replace").encode()
            self.send(200, "text/plain; charset=utf-8", body)
        elif path == "/api/ratings.csv":
            self.send(200, "text/csv; charset=utf-8", app.ratings.export_csv().encode(),
                      filename="bokehforge-ratings.csv")
        else:
            raise KeyError("Page not found")

    def send_events(self, app):
        self.close_connection = True
        self.connection.setsockopt(SOL_SOCKET, SO_KEEPALIVE, 1)
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream; charset=utf-8")
        self.send_header("Cache-Control", "no-store, no-transform")
        self.send_header("Connection", "close")
        self.send_header("X-Accel-Buffering", "no")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        self.end_headers()
        try:
            app.events.stream(self.connection, self.wfile.write)
        except (BrokenPipeError, ConnectionResetError, TimeoutError):
            pass

    def send_image(self, app, query):
        variant, identity, read = app.image_target(query)
        key = app.image_key(variant, identity)
        etag = f'"{key}"'
        cache = ("private, max-age=86400, immutable" if query.get("v") == app.image_version
                 else "private, max-age=0, must-revalidate")
        validators = self.headers.get("If-None-Match", "").split(",")
        if any(value.strip().removeprefix("W/") in {etag, "*"} for value in validators):
            self.send(304, preview_mime(variant), b"", cache=cache, etag=etag)
            return
        background = self.headers.get("X-BokehForge-Prefetch") == "1"
        body = app.image_body(key, variant, identity, read, background)
        self.send(200, preview_mime(variant), body, cache=cache, etag=etag)

    def post(self, path, _query):
        if self.headers.get_content_type() != "application/json":
            self.json(415, {"error": "Send an application/json request"})
            return
        size = int(self.headers.get("Content-Length", 0))
        if not 0 < size <= 32768:
            raise ValueError("Invalid request size")
        form = json.loads(self.rfile.read(size))
        if not isinstance(form, dict):
            raise ValueError("Expected a JSON object")
        app = self.server.app
        if path == "/api/runs":
            try:
                run_id = app.start(form)
            except CatalogPending:
                raise
            except RuntimeError as exc:
                self.json(409, {"error": str(exc)})
                return
            self.json(201, {"id": run_id})
        elif path == "/api/download":
            try:
                status = app.start_download()
            except RuntimeError as exc:
                self.json(409, {"error": str(exc)})
                return
            self.json(202, status)
        elif path == "/api/cancel":
            app.runs.cancel(form["run"])
            self.json(200, {"stopping": True})
        elif path == "/api/ratings":
            self.json(200, app.rate(form))
        else:
            raise KeyError("Page not found")

    def json(self, status, payload):
        self.send(status, "application/json; charset=utf-8", json.dumps(payload).encode())

    def send(self, status, mime, body, filename=None, *, cache="no-store", etag=None):
        self.send_response(status)
        if status != 304:
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", cache)
        if self.close_connection:
            self.send_header("Connection", "close")
        if etag is not None:
            self.send_header("ETag", etag)
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        self.send_header("Content-Security-Policy",
                         "default-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")
        if filename:
            self.send_header("Content-Disposition", f'attachment; filename="{filename}"')
        self.end_headers()
        self.wfile.write(body)

    def log_request(self, code="-", size="-"):
        if str(code).startswith("5"):
            super().log_request(code, size)


class ReviewServer(ThreadingHTTPServer):
    def __init__(self, address):
        # Serve one asset snapshot for the lifetime of the Python process, even
        # if the checkout is updated while this version is running.
        self.assets = {url: (mime, (STATIC / filename).read_bytes())
                       for url, (filename, mime) in ASSETS.items()}
        self.hostname = gethostname().lower()
        self.review_hosts = {self.hostname, address[0].lower()} - {ALL_INTERFACES}
        super().__init__(address, Handler)


    def server_close(self):
        if hasattr(self, "app"):
            self.app.events.close()
        super().server_close()


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge review",
                                     description="Browse and rate focus stacks on this computer")
    parser.add_argument("--root", type=Path, default=REPO / "datasets",
                        help="BokehForge dataset root (default: datasets/ in the repository)")
    parser.add_argument("--store", type=Path, default=REPO / "artifacts/bokehforge-review",
                        help="Local evaluations and ratings directory")
    parser.add_argument("--host", default=ALL_INTERFACES,
                        help="IPv4 bind address (default: 0.0.0.0, all interfaces)")
    parser.add_argument("--port", type=int, default=8765, help="HTTP port (default: 8765)")
    parser.add_argument("--dataset", action="append", help="Limit to a dataset ID, repeatable")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="Dataset source configuration")
    parser.add_argument("--lock", type=Path, help="Optional source snapshot")
    parser.add_argument("--pancake", help="Path to pancake-cli")
    parser.add_argument("--focus-stack", help="Path to focus-stack")
    parser.add_argument("--openfocus", help="Path to openfocus")
    parser.add_argument("--shine-python", help="Path to a Python with ShineStacker installed")
    args = parser.parse_args(argv)
    args.root = args.root.resolve()
    args.config = args.config.resolve()
    if args.lock is not None:
        args.lock = args.lock.resolve()
    args.store = args.store.resolve()
    with lock_store(args.store), ReviewServer((args.host, args.port)) as server:
        server.app = Review(args)
        port = server.server_port
        if args.host == ALL_INTERFACES:
            print(f"BokehForge Review · http://{server.hostname}:{port} · http://127.0.0.1:{port}", flush=True)
        else:
            print(f"BokehForge Review · http://{args.host}:{port}", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            server.app.downloads.close()
            server.app.runs.close()
    return 0
