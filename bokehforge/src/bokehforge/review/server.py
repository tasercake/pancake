"""Serve Stackroom over HTTP."""

import argparse
from contextlib import suppress
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
from ..bench import has_benchmark_reference
from ..data._canonical import RETIRED_LAYOUTS
from ..data._config import DEFAULT_CONFIG, read_config
from ..data._files import safe_path
from .downloads import Downloads
from .events import Events
from .images import (
    DecodedFrames, PREVIEW_VERSION, PreviewCache, ReviewFrameDecoder, SIZES,
    output_identity, output_pixels, output_revision, preview, preview_mime,
)
from .ratings import Ratings
from .runs import Runs, lock_store
from .scores import Scores
from .stacker_setup import REPO
from .stackers import configured_stackers

STATIC = Path(__file__).with_name("static")
ASSETS = {"/": ("index.html", "text/html; charset=utf-8"),
          "/app.js": ("app.js", "text/javascript; charset=utf-8"),
          "/rail.js": ("rail.js", "text/javascript; charset=utf-8"),
          "/score-explanation.js": ("score-explanation.js", "text/javascript; charset=utf-8"),
          "/style.css": ("style.css", "text/css; charset=utf-8"),
          "/fonts/IBMPlexMono-Regular.ttf": ("fonts/IBMPlexMono-Regular.ttf", "font/ttf"),
          "/fonts/IBMPlexMono-SemiBold.ttf": ("fonts/IBMPlexMono-SemiBold.ttf", "font/ttf"),
          "/fonts/SpaceGrotesk-Regular.ttf": ("fonts/SpaceGrotesk-Regular.ttf", "font/ttf"),
          "/fonts/SpaceGrotesk-SemiBold.ttf": ("fonts/SpaceGrotesk-SemiBold.ttf", "font/ttf"),
          "/fonts/IBMPlexMono-OFL.txt": ("fonts/IBMPlexMono-OFL.txt", "text/plain; charset=utf-8"),
          "/fonts/SpaceGrotesk-OFL.txt": ("fonts/SpaceGrotesk-OFL.txt", "text/plain; charset=utf-8")}
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
        self.scores = Scores(self.publish_scores, attention=not getattr(args, "no_attention", False))
        self.image_version = token_hex(8)
        self.previews = PreviewCache(args.store)
        self.decoded = DecodedFrames()
        self.frame_decoder = ReviewFrameDecoder()
        self.downloads = Downloads(args.root, args.config or DEFAULT_CONFIG, args.lock,
                                   self.refresh_catalog, self.publish_download)
        self.publish_runs()
        self.publish_scores()
        self.events.publish("catalog", self.catalog_data)
        self.refresh_catalog()

    def catalog_data(self):
        return {**self.catalog_status, "stackers": self.runs.tooling(), "image_version": self.image_version}

    def publish_runs(self):
        self.events.publish("runs", self.runs.list)

    def publish_scores(self):
        self.events.publish("scores", lambda: self.scores.revision)

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
            raise RuntimeError("Wait for Stackroom to check the local datasets")
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
        benchmark_supported = has_benchmark_reference(scene)
        for result in outputs:
            ready = result["status"] == "ready"
            result["rating"] = ratings.get(result["stacker"]) if ready else None
            if not ready:
                continue
            directory = self.runs.directory(run_id)
            if benchmark_supported:
                benchmark = self.scores.get(catalog, scene, directory, result)
                result["benchmark"] = benchmark
                result["artifact_revision"] = benchmark.get("artifact_revision")
            else:
                with suppress(OSError):
                    result["artifact_revision"] = output_revision(output_identity(directory / result["image"]))
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

    def score_tiles(self, query):
        catalog = self.require_catalog()
        scene = catalog.scene(query["scene"])
        if not has_benchmark_reference(scene):
            raise KeyError("Benchmark tiles require a supported RGB ground-truth image")
        run_id = query["run"]
        result = next((row for row in self.runs.scene_results(run_id, catalog, scene)
                       if row["stacker"] == query["stacker"] and row["status"] == "ready"), None)
        if result is None:
            raise KeyError("Evaluation output unavailable")
        return self.scores.tiles(catalog, scene, self.runs.directory(run_id), result,
                                 query["rev"], query["family"], int(query["side"]))

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
            identity = output_identity(path)
            if "rev" in query and query["rev"] != output_revision(identity):
                raise ValueError("The evaluation image changed. Reload its scene for the current version.")
            def read():
                pixels = output_pixels(path)
                if output_identity(path) != identity:
                    raise ValueError("The evaluation image changed while loading.")
                return pixels
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
        elif path == "/api/score-tiles":
            result = app.score_tiles(query)
            self.json(202 if result["status"] == "pending" else 200, result)
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
                      filename="stackroom-ratings.csv")
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
            self.app.scores.close()
            self.app.events.close()
        super().server_close()


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge review",
                                     description="Stackroom, focus stack review by BokehForge")
    workspace = parser.add_argument_group("Workspace")
    workspace.add_argument("--root", type=Path, default=REPO / "datasets",
                           help="BokehForge dataset root (default: datasets/ in the repository)")
    workspace.add_argument("--store", type=Path, default=REPO / "artifacts/bokehforge-review",
                           help="Local evaluations and ratings directory")
    workspace.add_argument("--dataset", action="append", help="Limit to a dataset ID, repeatable")
    workspace.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="Dataset source configuration")
    workspace.add_argument("--lock", type=Path, help="Optional source snapshot")
    workspace.add_argument("--no-attention", action="store_true", help="Disable inspection heatmaps and damage weighting")
    connection = parser.add_argument_group("Connection")
    connection.add_argument("--host", default=ALL_INTERFACES,
                            help="IPv4 bind address (default: 0.0.0.0, all interfaces)")
    connection.add_argument("--port", type=int, default=8765, help="HTTP port (default: 8765)")
    stackers = parser.add_argument_group("Stackers")
    stackers.add_argument("--pancake", help="Path to pancake-cli")
    stackers.add_argument("--focus-stack", help="Path to focus-stack")
    stackers.add_argument("--openfocus", help="Path to openfocus")
    stackers.add_argument("--shine-python", help="Path to a Python with ShineStacker installed")
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
            print(f"Stackroom · http://{server.hostname}:{port} · http://127.0.0.1:{port}", flush=True)
        else:
            print(f"Stackroom · http://{args.host}:{port}", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
        finally:
            server.app.downloads.close()
            server.app.runs.close()
    return 0
