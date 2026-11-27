"""Score saved review outputs at native resolution without blocking scene loading."""

from base64 import b64decode, b64encode
from collections import OrderedDict
import json
from threading import Condition, Lock, Thread
from zlib import decompress
from zlib import error as ZlibError

import numpy as np
from PIL import Image

from ..alignment import ALIGNMENT_VERSION
from ..bench import score_saved_output, write_score_explanation
from ..inspection import Inspection
from ..scoring import FAMILY_WEIGHTS, INSPECTION_SCORE_VERSION, SCORE_EXPLANATION_VERSION, SCORE_VERSION
from .images import output_identity, output_revision

_INVALID_DETAIL_VERSION = 1
_OVERLAY_GRIDS = {8: "8:0:0", 32: "32:0:0"}


class Scores:
    def __init__(self, on_change, *, attention=True):
        self.on_change = on_change
        self.inspection = Inspection() if attention else None
        self.profile = INSPECTION_SCORE_VERSION if attention else SCORE_VERSION
        self.revision = 0
        self.condition = Condition()
        self.compute = Lock()
        self.pending = OrderedDict()
        self.recent = OrderedDict()
        self.spatial = OrderedDict()
        self.attention_maps = OrderedDict()
        self.active = None
        self.closed = False
        self.worker = None

    def _cache(self, detail):
        if detail == "summary":
            return self.recent, 128
        if detail == "tiles":
            return self.spatial, 2
        return self.attention_maps, 4

    def _identity(self, scene, result, artifact):
        return (self.profile, ALIGNMENT_VERSION, scene.metadata["recipe_sha256"], result["scene_fingerprint"],
                *artifact[1:])

    def _measure(self, catalog, scene, image, detail):
        with self.compute:
            return score_saved_output(catalog, scene, image, details=detail,
                                      attention=self.inspection is not None, inspection=self.inspection)

    def record(self, catalog, scene, directory, row):
        """Score a new delivery and publish its complete diagnostic before its result row."""
        try:
            image = directory / row["image"]
            artifact = output_identity(image)
            result = self._measure(catalog, scene, image, "full")
            if output_identity(image) != artifact:
                return {"score_explanation_error": "The output changed while it was being scored."}
            if result["status"] not in {"ok", "invalid_output"}:
                return {"score_explanation_error": result.get("error") or "Scoring did not complete."}
            fields = {"recorded_benchmark": {
                "identity": list(self._identity(scene, row, artifact)),
                "result": self._compact_summary(result),
            }}
            if "explanation" in result:
                path = directory / row["stacker"] / "score-explanations" / (scene.id + ".json")
                write_score_explanation(path, result["explanation"])
                fields["score_explanation_path"] = path.relative_to(directory).as_posix()
            return fields
        except Exception as exc:
            # The TIFF remains a usable review output when scoring or exporting fails.
            return {"score_explanation_error": str(exc)}

    @staticmethod
    def _compact_summary(result):
        if "explanation" not in result:
            return result
        explanation = {**result["explanation"], "detail_level": "summary"}
        if "attention" in explanation:
            explanation["attention"] = {key: value for key, value in explanation["attention"].items()
                                        if key not in {"data", "encoding"}}
        placements = []
        for placement in explanation["placements"]:
            comparison = placement["comparison"]
            if comparison is not None:
                comparison = dict(comparison)
                comparison.pop("tile_grids", None)
                families = {}
                for name, family in comparison["families"].items():
                    pool = dict(family["pool"])
                    tail = dict(pool["tail"])
                    tail.pop("tile_indices", None)
                    pool["tail"] = tail
                    families[name] = {**family, "pool": pool}
                comparison["families"] = families
            placements.append({**placement, "comparison": comparison})
        explanation["placements"] = placements
        return {**result, "explanation": explanation}

    def get(self, catalog, scene, directory, result, *, detail="summary", expected_revision=None, map_digest=None):
        image = directory / result["image"]
        receipt = directory / result["stacker"] / "scores" / f"{scene.id}.json"
        try:
            artifact = output_identity(image)
        except OSError as exc:
            return {"profile": self.profile, "status": "missing_output", "score": None, "error": str(exc)}
        revision = output_revision(artifact)
        fields = {"profile": self.profile, "score": None, "artifact_revision": revision}
        if expected_revision is not None and expected_revision != revision:
            return {**fields, "status": "stale"}
        identity = self._identity(scene, result, artifact)
        recorded = result.get("recorded_benchmark")
        diagnostic = None
        if recorded and recorded["identity"] == list(identity):
            saved = recorded["result"]
            schema = saved.get("explanation", {}).get("schema_version")
            if saved["status"] == "invalid_output" or schema == SCORE_EXPLANATION_VERSION:
                if detail == "summary":
                    return {**saved, "artifact_revision": revision}
                if result.get("score_explanation_path"):
                    diagnostic = directory / result["score_explanation_path"]
        if map_digest is not None:
            identity = (*identity, map_digest)
        key = (receipt, identity, detail)
        cache, _ = self._cache(detail)
        with self.condition:
            if key in cache:
                cache.move_to_end(key)
                return {**cache[key], "artifact_revision": revision}
            if detail == "summary" and self.inspection is None:
                saved = self._read_summary(receipt, identity)
                if saved is not None:
                    return {**saved, "artifact_revision": revision}
            if not self.closed and self.active != key:
                self.pending[key] = (catalog, scene, image, artifact, map_digest, diagnostic)
                self.pending.move_to_end(key)
                if self.worker is None:
                    self.worker = Thread(target=self._work, name="bokehforge-review-scores", daemon=True)
                    self.worker.start()
                self.condition.notify()
        return {**fields, "status": "pending"}

    @staticmethod
    def _read_summary(receipt, identity):
        try:
            saved = json.loads(receipt.read_text(encoding="utf-8"))
            result = saved["result"]
            if saved["identity"] != list(identity):
                return None
            if result["status"] == "invalid_output" and saved.get("invalid_detail_version") != _INVALID_DETAIL_VERSION:
                return None
            if result["status"] == "ok" and result.get("explanation", {}).get("schema_version") != SCORE_EXPLANATION_VERSION:
                return None
            return result
        except (OSError, ValueError, KeyError, TypeError, AttributeError):
            return None

    def tiles(self, catalog, scene, directory, result, revision, family, side):
        if family not in FAMILY_WEIGHTS or side not in _OVERLAY_GRIDS:
            raise ValueError("Unknown damage family or grid size")
        spatial = self.get(catalog, scene, directory, result, detail="tiles", expected_revision=revision)
        grids = spatial.pop("grids", None)
        if grids is None:
            return spatial
        grid = grids[_OVERLAY_GRIDS[side]]
        return {**spatial, "family": family,
                "grid": {**grid, "values": grid["values"][family]}}

    def attention_overlay(self, catalog, scene, directory, result, revision):
        summary = self.get(catalog, scene, directory, result, expected_revision=revision)
        if summary["status"] != "ok":
            return summary
        explanation = summary.get("explanation") or {}
        placements = explanation.get("placements", [])
        digest = summary.get("inspection_map_sha256")
        if not (self.inspection is not None and digest and explanation.get("attention") and placements):
            return {"status": "no_overlay"}
        response = self.get(catalog, scene, directory, result, detail="attention",
                            expected_revision=revision, map_digest=digest)
        if response["status"] != "ok":
            return response
        return {**response, **self._overlay_geometry(explanation, placements[0])}

    def _attention_map(self, catalog, scene, expected_digest):
        raw = self.inspection.cached_map(expected_digest)
        if raw is None:
            reference = catalog.decode(scene.reference_ids[0]).data
            raw, digest = self.inspection.preview_map(reference)
            if digest != expected_digest:
                return {"status": "stale"}
        return self._attention_preview(raw, expected_digest)

    @staticmethod
    def _attention_preview(raw, digest):
        # Quantize only for display. The saved diagnostic retains native float32 values.
        values = np.rint(raw * 255).astype(np.uint8)
        return {"status": "ok", "inspection_map_sha256": digest,
                "attention": {"size_wh": list(values.shape[::-1]), "encoding": "base64+uint8",
                              "data": b64encode(values.tobytes()).decode("ascii")}}

    def _read_overlay(self, path, detail, digest):
        if path is None:
            return None
        try:
            explanation = json.loads(path.read_text(encoding="utf-8"))
            if explanation["schema_version"] != SCORE_EXPLANATION_VERSION or explanation["detail_level"] != "full":
                return None
            if detail == "tiles":
                result = {"status": "ok", "profile": explanation["profile"], "score": explanation["score"],
                          "explanation": explanation}
                if "inspection_map_sha256" in explanation:
                    result["inspection_map_sha256"] = explanation["inspection_map_sha256"]
                return result
            if explanation.get("inspection_map_sha256") != digest:
                return None
            attention = explanation["attention"]
            width, height = attention["size_wh"]
            pixels = decompress(b64decode(attention["data"]))
            image = Image.fromarray(np.frombuffer(pixels, dtype="<f4").reshape(height, width))
            image.thumbnail((640, 640), Image.Resampling.BILINEAR)
            return self._attention_preview(np.asarray(image), digest)
        except (OSError, ValueError, TypeError, KeyError, ZlibError):
            return None

    def _work(self):
        while True:
            with self.condition:
                self.condition.wait_for(lambda: self.pending or self.closed)
                if self.closed:
                    return
                # Score the currently requested scene before older ones. Run
                # one native comparison at a time to limit memory use.
                key, (catalog, scene, image, artifact, map_digest, diagnostic) = self.pending.popitem()
                self.active = key
            detail = key[2]
            try:
                result = self._read_overlay(diagnostic, detail, map_digest)
                if result is None and detail == "attention":
                    with self.compute:
                        result = self._attention_map(catalog, scene, map_digest)
                if result is None:
                    result = self._measure(catalog, scene, image, detail)
                if output_identity(image) != artifact:
                    result = {"profile": self.profile, "status": "stale", "score": None}
                elif detail == "tiles":
                    result = self._spatial_result(result)
            except Exception as exc:
                result = {"profile": self.profile, "status": "scoring_error", "score": None, "error": str(exc)}
            if detail == "summary" and self.inspection is None and result["status"] in {"ok", "invalid_output"}:
                self._save(key, result)
            with self.condition:
                cache, limit = self._cache(detail)
                cache[key] = result
                if len(cache) > limit:
                    cache.popitem(last=False)
                self.active = None
                self.revision += 1
            self.on_change()

    @staticmethod
    def _overlay_geometry(explanation, placement):
        return {"reference_id": explanation["reference_id"],
                "reference_size_wh": explanation["reference_size_wh"],
                "candidate_size_wh": explanation["candidate_size_wh"],
                "candidate_to_reference_xy": placement["candidate_to_reference_xy"],
                "candidate_content_box_xyxy": placement["candidate_content_box_xyxy"]}

    @staticmethod
    def _spatial_result(result):
        explanation = result.pop("explanation", None)
        if explanation is None:
            return result
        placements = explanation["placements"]
        if not placements or placements[0]["comparison"] is None:
            return {**result, "status": "no_overlay"}
        placement = placements[0]
        grids = placement["comparison"]["tile_grids"]
        return {**result, **Scores._overlay_geometry(explanation, placement),
                "damage_range": placement["comparison"]["damage_range"],
                "grids": {key: grids[key] for key in _OVERLAY_GRIDS.values()}}

    @staticmethod
    def _save(key, result):
        receipt, identity, _ = key
        pending = receipt.with_suffix(".tmp")
        try:
            receipt.parent.mkdir(parents=True, exist_ok=True)
            saved = {"identity": identity, "result": result, "invalid_detail_version": _INVALID_DETAIL_VERSION}
            pending.write_text(json.dumps(saved) + "\n", encoding="utf-8")
            pending.replace(receipt)
        except OSError:
            # The score can still be displayed from memory with a read-only store.
            pass

    def close(self):
        with self.condition:
            self.closed = True
            self.pending.clear()
            self.condition.notify()
        if self.worker is not None:
            self.worker.join()
