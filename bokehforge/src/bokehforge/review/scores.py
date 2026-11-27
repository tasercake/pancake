"""Score saved review outputs at native resolution without blocking scene loading."""

from collections import OrderedDict
import json
from threading import Condition, Thread

from ..alignment import ALIGNMENT_VERSION
from ..bench import score_saved_output
from ..inspection import map_path
from ..scoring import FAMILY_WEIGHTS, INSPECTION_SCORE_VERSION, SCORE_EXPLANATION_VERSION, SCORE_VERSION
from .images import output_identity, output_revision

_INVALID_DETAIL_VERSION = 1
_OVERLAY_GRIDS = {8: "8:0:0", 32: "32:0:0"}


class Scores:
    def __init__(self, on_change, inspection_maps=None):
        self.on_change = on_change
        self.inspection_maps = inspection_maps
        self.profile = SCORE_VERSION if inspection_maps is None else INSPECTION_SCORE_VERSION
        self.revision = 0
        self.condition = Condition()
        self.pending = OrderedDict()
        self.recent = OrderedDict()
        self.spatial = OrderedDict()
        self.active = None
        self.closed = False
        self.worker = None

    def get(self, catalog, scene, directory, result, *, detail="summary", expected_revision=None):
        image = directory / result["image"]
        receipt = directory / result["stacker"] / (
            "scores" if self.inspection_maps is None else "inspection-scores"
        ) / f"{scene.id}.json"
        try:
            artifact = output_identity(image)
        except OSError as exc:
            return {"profile": self.profile, "status": "missing_output", "score": None, "error": str(exc)}
        revision = output_revision(artifact)
        fields = {"profile": self.profile, "score": None, "artifact_revision": revision}
        if expected_revision is not None and expected_revision != revision:
            return {**fields, "status": "stale"}
        map_artifact = ()
        if self.inspection_maps is not None:
            try:
                map_artifact = output_identity(map_path(self.inspection_maps, scene))
            except OSError as exc:
                return {**fields, "status": "scoring_error", "error": str(exc)}
        identity = (self.profile, ALIGNMENT_VERSION, scene.metadata["recipe_sha256"], result["scene_fingerprint"],
                    *artifact[1:], *map_artifact)
        key = (receipt, identity, detail)
        cache = self.recent if detail == "summary" else self.spatial
        with self.condition:
            if key in cache:
                cache.move_to_end(key)
                return {**cache[key], "artifact_revision": revision}
            if detail == "summary":
                saved = self._read_summary(receipt, identity)
                if saved is not None:
                    return {**saved, "artifact_revision": revision}
            if not self.closed and self.active != key:
                self.pending[key] = (catalog, scene, image, artifact, map_artifact)
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

    def _work(self):
        while True:
            with self.condition:
                self.condition.wait_for(lambda: self.pending or self.closed)
                if self.closed:
                    return
                # Score the currently requested scene before older ones. Run
                # one native comparison at a time to limit memory use.
                key, (catalog, scene, image, artifact, map_artifact) = self.pending.popitem()
                self.active = key
            detail = key[2]
            try:
                options = {} if self.inspection_maps is None else {"inspection_maps": self.inspection_maps}
                result = score_saved_output(catalog, scene, image, details=detail, **options)
                if output_identity(image) != artifact or (
                    map_artifact and output_identity(map_path(self.inspection_maps, scene)) != map_artifact
                ):
                    result = {"profile": self.profile, "status": "stale", "score": None}
                elif detail == "tiles":
                    result = self._spatial_result(result)
            except Exception as exc:
                result = {"profile": self.profile, "status": "scoring_error", "score": None, "error": str(exc)}
            if detail == "summary" and result["status"] in {"ok", "invalid_output"}:
                self._save(key, result)
            with self.condition:
                cache, limit = (self.recent, 128) if detail == "summary" else (self.spatial, 2)
                cache[key] = result
                if len(cache) > limit:
                    cache.popitem(last=False)
                self.active = None
                self.revision += 1
            self.on_change()

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
        return {**result, "reference_id": explanation["reference_id"],
                "damage_range": placement["comparison"]["damage_range"],
                "reference_size_wh": explanation["reference_size_wh"],
                "candidate_size_wh": explanation["candidate_size_wh"],
                "candidate_to_reference_xy": placement["candidate_to_reference_xy"],
                "candidate_content_box_xyxy": placement["candidate_content_box_xyxy"],
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
