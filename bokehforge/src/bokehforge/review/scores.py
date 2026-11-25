"""Score saved review outputs at native resolution without blocking scene loading."""

from collections import OrderedDict
import json
from threading import Condition, Thread

from ..bench import has_benchmark_reference, score_saved_output
from ..scoring import SCORE_VERSION

_INVALID_DETAIL_VERSION = 1


class Scores:
    def __init__(self, on_change):
        self.on_change = on_change
        self.revision = 0
        self.condition = Condition()
        self.pending = OrderedDict()
        self.recent = OrderedDict()
        self.active = None
        self.closed = False
        self.worker = None

    def get(self, catalog, scene, directory, result):
        if not has_benchmark_reference(scene):
            return {"profile": SCORE_VERSION, "status": "unscored", "score": None}
        image = directory / result["image"]
        receipt = directory / result["stacker"] / "scores" / f"{scene.id}.json"
        try:
            stat = image.stat()
        except OSError as exc:
            return {"profile": SCORE_VERSION, "status": "missing_output", "score": None, "error": str(exc)}
        identity = (SCORE_VERSION, scene.metadata["recipe_sha256"], result["scene_fingerprint"],
                    stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns)
        key = (receipt, identity)
        with self.condition:
            if key in self.recent:
                self.recent.move_to_end(key)
                return self.recent[key]
            try:
                saved = json.loads(receipt.read_text(encoding="utf-8"))
                result = saved["result"]
                current_details = (result["status"] != "invalid_output"
                                   or saved.get("invalid_detail_version") == _INVALID_DETAIL_VERSION)
                if saved["identity"] == list(identity) and current_details:
                    return result
            except (OSError, ValueError, KeyError, TypeError):
                pass
            if not self.closed and self.active != key:
                self.pending[key] = (catalog, scene, image)
                self.pending.move_to_end(key)
                if self.worker is None:
                    self.worker = Thread(target=self._work, name="bokehforge-review-scores", daemon=True)
                    self.worker.start()
                self.condition.notify()
        return {"profile": SCORE_VERSION, "status": "pending", "score": None}

    def _work(self):
        while True:
            with self.condition:
                self.condition.wait_for(lambda: self.pending or self.closed)
                if self.closed:
                    return
                # Score the currently requested scene before older ones. Run
                # one native comparison at a time to limit memory use.
                key, (catalog, scene, image) = self.pending.popitem()
                self.active = key
            try:
                result = score_saved_output(catalog, scene, image)
            except Exception as exc:
                result = {"profile": SCORE_VERSION, "status": "scoring_error", "score": None, "error": str(exc)}
            if result["status"] in {"ok", "invalid_output"}:
                self._save(key, result)
            with self.condition:
                self.recent[key] = result
                if len(self.recent) > 128:
                    self.recent.popitem(last=False)
                self.active = None
                self.revision += 1
            self.on_change()

    @staticmethod
    def _save(key, result):
        receipt, identity = key
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
