"""Serve the BokehForge rating app over HTTP."""

import argparse
from functools import lru_cache
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from ipaddress import ip_address
import json
from pathlib import Path
from socket import gethostname
from threading import BoundedSemaphore, Thread
from urllib.parse import parse_qs, urlsplit

from .. import SceneCatalog
from .images import output_pixels, preview
from .ratings import Ratings
from .runs import REPO, Runs, configured_stackers, lock_store

STATIC = Path(__file__).with_name("static")
ASSETS = {"/": ("index.html", "text/html; charset=utf-8"),
          "/app.js": ("app.js", "text/javascript; charset=utf-8"),
          "/style.css": ("style.css", "text/css; charset=utf-8")}
SIZES = {"thumb": 192, "fit": 1500, "native": 0}
ALL_INTERFACES = "0.0.0.0"


def dataset_names(root, lock, dataset_ids):
    if lock is not None:
        sources = json.loads(lock.read_text())["datasets"]
    else:
        manifests = (root / "raw" / dataset_id / "download-manifest.json"
                     for dataset_id in dataset_ids)
        sources = [json.loads(path.read_text()) for path in manifests if path.is_file()]
    names = {source["id"]: source.get("name") for source in sources}
    return {dataset_id: names.get(dataset_id) or dataset_id for dataset_id in dataset_ids}


class CatalogPending(RuntimeError):
    pass


class Review:
    def __init__(self, args):
        self.catalog = None
        self.catalog_status = {"status": "indexing", "root": str(args.root)}
        self.scenes = []
        self.runs = Runs(args.store, configured_stackers(args))
        self.ratings = Ratings(args.store)
        self.decode_slots = BoundedSemaphore(2)
        self.cached_preview = lru_cache(maxsize=40)(self._preview)
        Thread(target=self._load, args=(args,), daemon=True, name="bokehforge-catalog").start()

    def _load(self, args):
        try:
            catalog = SceneCatalog(args.root, datasets=args.dataset, config=args.config, lock=args.lock)
            scenes = [catalog.scene(scene_id) for scene_id in catalog.scene_ids]
            self.scenes = [
                {"id": scene.id, "name": scene.metadata["source_scene_id"],
                 "dataset": scene.metadata["dataset_id"], "frames": len(scene.frame_ids)}
                for scene in scenes
            ]
            datasets = dataset_names(args.root, args.lock, dict.fromkeys(scene["dataset"] for scene in self.scenes))
            self.catalog = catalog
            self.catalog_status = {"status": "ready", "scenes": self.scenes, "datasets": datasets}
        except Exception as exc:
            self.catalog_status = {**self.catalog_status, "status": "error", "error": str(exc)}

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

    def _preview(self, name, pixels, size, thumbnail):
        return preview(pixels(name), size, thumbnail)

    def image(self, query):
        catalog = self.require_catalog()
        variant = query.get("size", "fit")
        size = SIZES[variant]
        thumbnail = variant == "thumb"
        if "frame" in query:
            name = query["frame"]
            catalog.frame(name)
            pixels = self.frame_pixels
        else:
            name = self.artifact(query, "image")
            pixels = output_pixels
        with self.decode_slots:
            render = self.cached_preview if size else self._preview
            return ("image/jpeg" if thumbnail else "image/png"), render(name, pixels, size, thumbnail)

    def frame_pixels(self, frame_id):
        return self.catalog.decode(frame_id).data

    def start(self, form):
        catalog = self.require_catalog()
        kind = form["scope"]
        if kind == "scene":
            scene = catalog.scene(form["scene"])
            scene_ids = [scene.id]
            label = scene.metadata["source_scene_id"]
        elif kind == "dataset":
            label = form["dataset"]
            scene_ids = [scene["id"] for scene in self.scenes if scene["dataset"] == label]
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
        return self.runs.start(catalog, scene_ids, stackers, {"type": kind, "label": label})

    def rate(self, form):
        self.artifact(form, "image")
        return self.ratings.save(form["run"], form["scene"], form["stacker"], form["grade"],
                                 form.get("comment", ""))


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.handle_request(self.get)

    def do_POST(self):
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
        app = self.server.app
        if path in ASSETS:
            filename, mime = ASSETS[path]
            self.send(200, mime, (STATIC / filename).read_bytes())
        elif path == "/api/catalog":
            self.json(200, {**app.catalog_status, "stackers": app.runs.tooling()})
        elif path == "/api/runs":
            self.json(200, app.runs.list())
        elif path == "/api/scene":
            self.json(200, app.scene_details(query["scene"], query.get("run")))
        elif path == "/api/image":
            mime, body = app.image(query)
            self.send(200, mime, body)
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
        elif path == "/api/cancel":
            app.runs.cancel(form["run"])
            self.json(200, {"stopping": True})
        elif path == "/api/ratings":
            self.json(200, app.rate(form))
        else:
            raise KeyError("Page not found")

    def json(self, status, payload):
        self.send(status, "application/json; charset=utf-8", json.dumps(payload).encode())

    def send(self, status, mime, body, filename=None):
        self.send_response(status)
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
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
        self.hostname = gethostname().lower()
        self.review_hosts = {self.hostname, address[0].lower()} - {ALL_INTERFACES}
        super().__init__(address, Handler)


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge review",
                                     description="Browse and rate focus stacks on this computer")
    parser.add_argument("--root", type=Path, default=REPO / "datasets", help="BokehForge dataset root")
    parser.add_argument("--store", type=Path, default=REPO / "artifacts/bokehforge-review",
                        help="Local evaluations and ratings directory")
    parser.add_argument("--host", default=ALL_INTERFACES,
                        help="IPv4 bind address (default: 0.0.0.0, all interfaces)")
    parser.add_argument("--port", type=int, default=8765, help="HTTP port (default: 8765)")
    parser.add_argument("--dataset", action="append", help="Limit to a dataset ID, repeatable")
    parser.add_argument("--config", type=Path, help="Optional catalog policy overrides")
    parser.add_argument("--lock", type=Path, help="Optional source snapshot")
    parser.add_argument("--pancake", help="Path to pancake-cli")
    parser.add_argument("--focus-stack", help="Path to focus-stack")
    parser.add_argument("--openfocus", help="Path to openfocus")
    parser.add_argument("--shine-python", help="Path to a Python with ShineStacker installed")
    args = parser.parse_args(argv)
    args.root = args.root.resolve()
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
            server.app.runs.close()
    return 0
