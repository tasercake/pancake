#!/usr/bin/env python3
"""Resolve pinned dataset sources and acquire checksum-verified original files."""

import argparse
import concurrent.futures
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sys
import threading
import time
from urllib.parse import quote, urlparse

import requests

from ._canonical import RETIRED_LAYOUTS
from ._config import DEFAULT_CONFIG, data_root, read_config
from ._files import digest, natural_key, safe_path
from .layout import METADATA, data_run, local_source, matches_pins

MUTEX = threading.Lock()


def save_json(path, value):
    payload = (json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n").encode()
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_bytes() == payload:
        return
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(payload)
    temporary.replace(path)


def session():
    client = requests.Session()
    client.headers["User-Agent"] = "focus-stacking-reproducible-data/1.0"
    return client


def get_json(client, url):
    headers = {}
    token = os.environ.get("GITHUB_TOKEN")
    if token and urlparse(url).netloc == "api.github.com":
        headers["Authorization"] = f"Bearer {token}"
    response = client.get(url, headers=headers, timeout=(20, 90))
    response.raise_for_status()
    return response.json()


def drive_url(client, file_id):
    """Resolve Google's public download form without accepting account access gates."""
    from bs4 import BeautifulSoup
    url = f"https://drive.google.com/uc?export=download&id={file_id}"
    for _ in range(3):
        with client.get(url, stream=True, timeout=(20, 90)) as response:
            response.raise_for_status()
            if "text/html" not in response.headers.get("Content-Type", ""):
                return response.url
            html = response.raw.read(1024 * 1024, decode_content=True).decode("utf-8", "replace")
        soup = BeautifulSoup(html, "html.parser")
        form = soup.find("form", id="download-form")
        if not form:
            detail = " ".join(soup.stripped_strings)[:450]
            raise RuntimeError(f"Google Drive download unavailable at {url}: {detail}")
        from urllib.parse import urlencode, urljoin
        params = [(tag.get("name"), tag.get("value", "")) for tag in form.find_all("input") if tag.get("name")]
        url = urljoin(response.url, form.get("action", "")) + "?" + urlencode(params)
    raise RuntimeError("Google Drive download form did not resolve after three requests")


def resolve_source(client, source):
    result = dict(source)
    provider = source.get("provider", "unavailable")
    if provider in {"unavailable", "disabled"}:
        result.update(status=provider, files=[])
        return result
    files = []
    if "files" in source:
        files = source["files"]
    elif provider == "github":
        repo = source["repository"]
        commit = get_json(client, f"https://api.github.com/repos/{repo}/commits/{source.get('ref', 'HEAD')}")["sha"]
        tree = get_json(client, f"https://api.github.com/repos/{repo}/git/trees/{commit}?recursive=1")
        if tree.get("truncated"):
            raise RuntimeError("GitHub tree truncated; refusing incomplete source inventory")
        for item in tree["tree"]:
            if item["type"] != "blob" or not re.search(source["include_pattern"], item["path"], re.I):
                continue
            files.append({"path": item["path"], "url": f"https://raw.githubusercontent.com/{repo}/{commit}/{quote(item['path'])}",
                          "size": item.get("size"), "git_blob_sha1": item["sha"], "sha256": None})
        result["revision"] = commit
    elif provider == "huggingface":
        repo, revision = source["repository"], source["revision"]
        metadata = get_json(client, f"https://huggingface.co/api/datasets/{repo}/revision/{revision}?blobs=true")
        for item in metadata["siblings"]:
            path = item["rfilename"]
            if not any(fnmatch.fnmatch(path, pattern) for pattern in source["include"]):
                continue
            lfs = item.get("lfs") or {}
            files.append({"path": path, "url": f"https://huggingface.co/datasets/{repo}/resolve/{revision}/{quote(path)}",
                          "size": item.get("size") or lfs.get("size"), "sha256": lfs.get("sha256"),
                          **({"git_blob_sha1": item["blobId"]} if not lfs and item.get("blobId") else {})})
    elif provider == "figshare":
        metadata = get_json(client, f"https://api.figshare.com/v2/articles/{source['article_id']}/versions/{source.get('version', 1)}")
        seen = set()
        for item in metadata["files"]:
            if not re.search(source["include_pattern"], item["name"], re.I):
                continue
            md5 = item.get("computed_md5") or item.get("supplied_md5")
            if (md5, item["size"]) in seen:
                continue
            seen.add((md5, item["size"]))
            files.append({"path": item["name"], "url": item["download_url"], "size": item["size"], "md5": md5, "sha256": None})
        result["revision"] = str(metadata["version"])
    else:
        raise ValueError(f"Unknown provider {provider}")
    if not files:
        raise RuntimeError("Source inventory is empty")
    paths = [item["path"] for item in files]
    if len(set(paths)) != len(paths):
        raise RuntimeError("Provider lists duplicate local paths; refusing ambiguous file identity")
    result.update(status="available", files=sorted(files, key=lambda x: natural_key(x["path"])))
    return result


def verified(path, item):
    if not path.is_file() or (item.get("size") is not None and path.stat().st_size != item["size"]):
        return False
    checks = [(key, algorithm) for key, algorithm in [("sha256", "sha256"), ("md5", "md5"), ("git_blob_sha1", "sha1")] if item.get(key)]
    if not checks or any(digest(path, algorithm) != item[key] for key, algorithm in checks):
        return False
    item["sha256"] = item.get("sha256") or digest(path)
    item["size"] = path.stat().st_size
    return True


def check_space(path, needed, reserve):
    path.mkdir(parents=True, exist_ok=True)
    free = shutil.disk_usage(path).free
    if free < needed + reserve:
        raise RuntimeError(f"Insufficient disk space: need {needed + reserve:,} bytes including reserve; {free:,} free")


def acquire(client, target, item, reserve, retries, *, progress=None):
    safe_path(target.parent, target.name)
    if progress:
        progress("checking", 0, item.get("size"))
    if verified(target, item):
        return False
    part = target.with_name(target.name + ".part")
    target.parent.mkdir(parents=True, exist_ok=True)
    if verified(part, item):
        part.replace(target)
        return True
    if part.exists() and item.get("size") is not None and part.stat().st_size >= item["size"]:
        part.unlink()
    check_space(target.parent, max(0, (item.get("size") or 0) - (part.stat().st_size if part.exists() else 0)), reserve)
    error = None
    for attempt in range(retries):
        try:
            url = drive_url(client, item["drive_id"]) if item.get("drive_id") else item["url"]
            offset = part.stat().st_size if part.exists() else 0
            headers = {"Accept-Encoding": "identity"}
            if offset:
                headers["Range"] = f"bytes={offset}-"
            response = client.get(url, headers=headers, stream=True, timeout=(20, 120))
            if offset and response.status_code == 416:
                # A crash can leave a complete but unhashed first download.
                # Without an independent checksum, safely fetch it from zero.
                response.close()
                part.unlink()
                offset = 0
                response = client.get(url, headers={"Accept-Encoding": "identity"}, stream=True, timeout=(20, 120))
            with response:
                response.raise_for_status()
                if "text/html" in response.headers.get("Content-Type", "") and Path(item["path"]).suffix.lower() not in {".html", ".htm", ".md", ".txt"}:
                    raise RuntimeError("Provider returned an HTML access/error page instead of dataset bytes")
                if offset and response.status_code == 206:
                    if not response.headers.get("Content-Range", "").startswith(f"bytes {offset}-"):
                        raise RuntimeError("Resume Content-Range mismatch")
                elif response.status_code == 200:
                    offset = 0
                elif response.status_code != 206:
                    raise RuntimeError(f"Unexpected download HTTP status {response.status_code}")
                expected = int(response.headers.get("Content-Length", 0))
                check_space(target.parent, expected, reserve)
                total = item.get("size") or (offset + expected if expected else None)
                received = offset
                updated = time.monotonic()
                if progress:
                    progress("downloading", received, total)
                with part.open("ab" if offset else "wb") as handle:
                    for chunk in response.iter_content(1024 * 1024):
                        if chunk:
                            check_space(target.parent, len(chunk), reserve)
                            handle.write(chunk)
                            received += len(chunk)
                            if progress and time.monotonic() - updated >= 0.2:
                                progress("downloading", received, total)
                                updated = time.monotonic()
            if progress:
                progress("downloading", received, total)
                progress("verifying", received, total)
            if item.get("size") is not None and part.stat().st_size != item["size"]:
                raise RuntimeError(f"Downloaded size mismatch for {item['path']}")
            checksum = digest(part)
            for key, algorithm in [("sha256", "sha256"), ("md5", "md5"), ("git_blob_sha1", "sha1")]:
                if item.get(key) and (checksum if key == "sha256" else digest(part, algorithm)) != item[key]:
                    part.unlink(missing_ok=True)
                    raise RuntimeError(f"Checksum mismatch ({key}) for {item['path']}")
            item["sha256"] = checksum
            item["size"] = part.stat().st_size
            part.replace(target)
            return True
        except (requests.RequestException, OSError, RuntimeError) as exc:
            if progress:
                progress("retrying", part.stat().st_size if part.is_file() else 0, item.get("size"))
            error = exc
            if "disk space" in str(exc) or "Range" in str(exc) or "checksum" in str(exc).lower():
                break
            if attempt + 1 < retries:
                time.sleep(min(attempt + 1, 3))
    raise RuntimeError(str(error))


def main(argv=None, *, progress=None):
    parser = argparse.ArgumentParser(prog="bokehforge data download", description=__doc__,
                                     epilog="Logs and reports are saved under <root>/.bokehforge/.")
    selection = parser.add_argument_group("Dataset selection")
    selection.add_argument("--root", type=Path, help="Dataset directory (default: the shared BokehForge data location)")
    selection.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    selection.add_argument("--lock", type=Path)
    selection.add_argument("--dataset", dest="datasets", action="append", help="Dataset ID; repeat to select several")
    resolution = parser.add_argument_group("Source resolution")
    resolution.add_argument(
        "--resolve-only", action="store_true", help="Resolve source files without downloading them",
    )
    resolution.add_argument("--refresh-lock", action="store_true", help="Explicitly resolve a new upstream snapshot")
    resolution.add_argument(
        "--probe-url", help="Record a public endpoint response without downloading a full binary body",
    )
    limits = parser.add_argument_group("Download limits")
    limits.add_argument("--reserve-gib", type=float, help="Minimum free space to retain in GiB")
    limits.add_argument("--workers", type=int, default=4, help="Concurrent downloads (default: 4)")
    limits.add_argument("--retries", type=int, default=3, help="Maximum attempts per file (default: 3)")
    return download_datasets(**vars(parser.parse_args(argv)), progress=progress)


def download_datasets(*, datasets=None, config=DEFAULT_CONFIG, root=None, lock=None, reserve_gib=None,
                      workers=4, retries=3, resolve_only=False, refresh_lock=False, probe_url=None,
                      progress=None, only_missing=False):
    """Acquire verified originals using the same store and events for all callers."""
    config = Path(config).resolve()
    configured = read_config(config)
    defaults = configured.get("defaults", {})
    root = Path(root).expanduser().resolve() if root is not None else data_root(config, defaults)
    bundled = config == DEFAULT_CONFIG and lock is None
    lock_path = (Path(lock).expanduser().resolve() if lock is not None else
                 (root / METADATA if bundled else config.parent) / "sources.lock.json")
    reserve_gib = reserve_gib if reserve_gib is not None else defaults.get("reserve_gib", 15)
    if workers < 1 or retries < 1 or reserve_gib < 0:
        raise ValueError("workers/retries must be positive and reserve must be nonnegative")
    sources = {d["id"]: d for d in configured["datasets"] if d.get("layout") not in RETIRED_LAYOUTS}
    selected = set(datasets or sources)
    if unknown := selected - sources.keys():
        raise ValueError(f"Unknown dataset IDs: {sorted(unknown)}")
    with data_run(root, "download") as (out, err):
        if probe_url:
            with session() as client, client.get(probe_url, stream=True, timeout=(20, 90)) as response:
                content_type = response.headers.get("Content-Type", "")
                textual = any(x in content_type for x in ["text/", "json", "xml"])
                record = {"url": probe_url, "resolved_url": response.url, "status": response.status_code,
                          "content_type": content_type, "content_length": response.headers.get("Content-Length"),
                          "text": response.raw.read(65536, decode_content=True).decode("utf-8", "replace") if textual else None}
                save_json(root / METADATA / "probes" / (hashlib.sha256(probe_url.encode()).hexdigest()[:16] + ".json"), record)
                print(json.dumps(record, indent=2), file=out)
                return 0
        if only_missing:
            selected = {key for key in selected if local_source(root, key, pins=sources[key]) is None}
            if not selected:
                return 0
        print(f"Preparing datasets {', '.join(sorted(selected))} in {root}", file=err, flush=True)
        lock = json.loads(lock_path.read_text()) if lock_path.exists() else {"schema_version": 1, "datasets": []}
        lock["datasets"] = [
            d for d in lock["datasets"]
            if d.get("layout") not in RETIRED_LAYOUTS
            and (d["id"] not in selected or (not refresh_lock and d["status"] != "resolve_failed"
                 and (not bundled or matches_pins(d, sources[d["id"]]))))
        ]
        disabled = {dataset_id: sources[dataset_id] for dataset_id in selected
                    if sources[dataset_id].get("provider") == "disabled"}
        lock["datasets"] = [{**disabled[d["id"]], "status": "disabled", "files": []}
                            if d["id"] in disabled else d for d in lock["datasets"]]
        save_json(lock_path, lock)
        ids = {d["id"] for d in lock["datasets"]}
        for dataset_id, source in sources.items():
            if dataset_id not in selected or dataset_id in ids:
                continue
            print(f"Resolving {dataset_id} ({source.get('name', dataset_id)})", file=err, flush=True)
            try:
                with session() as client:
                    resolved = resolve_source(client, source)
            except Exception as exc:
                reason = re.sub(r"&uuid=[^&\s:]+", "&uuid=<ephemeral>", str(exc))
                resolved = {**source, "status": "resolve_failed", "reason": reason, "files": []}
            lock["datasets"].append(resolved)
            print(f"RESOLVE {source['id']}: {resolved['status']} {len(resolved['files'])} files", file=out, flush=True)
            save_json(lock_path, lock)
        active = [d for d in lock["datasets"] if d["id"] in selected]
        if resolve_only:
            return 0 if all(d["status"] in {"available", "disabled"} for d in active) else 2
        if progress:
            progress({"type": "inventory", "files": [
                {"dataset": dataset["id"], "path": item["path"], "total": item.get("size")}
                for dataset in active if dataset["status"] == "available" for item in dataset["files"]
            ]})
        reports = []
        for dataset in active:
            record = {"id": dataset["id"], "status": dataset["status"], "files": len(dataset["files"]), "verified": 0, "errors": []}
            if dataset["status"] != "available":
                record["reason"] = dataset.get("reason", "No public source download route")
                if dataset["status"] != "disabled":
                    message = f"{dataset['id']}: {record['reason']}"
                    print(f"ERROR {message}", file=err, flush=True)
                    if progress:
                        progress({"type": "error", "message": message})
                reports.append(record)
                continue
            inputs = dataset["files"]
            errors = []
            def task(item):
                name = f"{dataset['id']}/{item['path']}"
                phase_shown = None
                last_update = 0

                def report(phase, received, total):
                    nonlocal phase_shown, last_update
                    if progress:
                        progress({"type": "file", "dataset": dataset["id"], "path": item["path"],
                                  "phase": phase, "received": received, "total": total})
                    now = time.monotonic()
                    if phase in ("complete", "failed"):
                        return
                    if phase == phase_shown and (phase != "downloading" or
                                                 (now - last_update < 5 and received != total)):
                        return
                    phase_shown, last_update = phase, now
                    count = f" {received / 1024**2:,.1f} MiB" if received else ""
                    if total is not None and phase == "downloading":
                        count = f" {received / 1024**2:,.1f} / {total / 1024**2:,.1f} MiB ({received / max(total, 1):.0%})"
                    print(f"{phase.capitalize()} {name}{count}", file=err, flush=True)

                try:
                    target = safe_path(root / "raw" / dataset["id"], item["path"])
                    acquired = dict(item)
                    with session() as client:
                        changed = acquire(client, target, acquired, int(reserve_gib * 1024**3), retries, progress=report)
                    report("complete", acquired["size"], acquired["size"])
                    with MUTEX:
                        if acquired != item or changed:
                            item.update(acquired)
                            save_json(lock_path, lock)
                        print(f"{'FETCH' if changed else 'VERIFY'} {name}", file=out, flush=True)
                except Exception as exc:
                    with MUTEX:
                        errors.append({"path": item["path"], "error": str(exc)})
                        if progress:
                            report("failed", 0, item.get("size"))
                            progress({"type": "error", "message": f"{dataset['id']}/{item['path']}: {exc}"})
                        print(f"ERROR {name}: {exc}", file=err, flush=True)
            with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as pool:
                list(pool.map(task, inputs))
            record.update(status="complete" if not errors else "incomplete", verified=len(inputs) - len(errors), errors=sorted(errors, key=lambda x: x["path"]))
            reports.append(record)
            if not errors:
                save_json(root / "raw" / dataset["id"] / "download-manifest.json", dataset)
                print(f"Ready {dataset['id']} ({dataset.get('name', dataset['id'])})", file=err, flush=True)
                if progress:
                    progress({"type": "dataset_ready"})
            save_json(root / METADATA / "reports/download-report.json", {"schema_version": 1, "datasets": reports})
        save_json(root / METADATA / "reports/download-report.json", {"schema_version": 1, "datasets": reports})
        return 0 if all(d["status"] in {"complete", "disabled"} for d in reports) else 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(2)
