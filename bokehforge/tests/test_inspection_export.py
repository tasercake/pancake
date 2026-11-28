import sys

import pytest

from bokehforge import _inspection_export as exporter
from bokehforge._attention_setup import model_directory


def test_cold_export_repairs_dangling_model_cache(tmp_path, monkeypatch):
    cache = tmp_path.resolve() / "inspection-models"
    missing_target = tmp_path / "removed-worktree" / "models"
    try:
        cache.symlink_to(missing_target, target_is_directory=True)
    except OSError as exc:
        if sys.platform == "win32" and exc.winerror == 1314:
            pytest.skip("Creating symlinks requires privileges on this Windows host")
        raise
    monkeypatch.setenv("BOKEHFORGE_INSPECTION_MODELS", str(cache))
    calls = []

    def export(source, destination):
        calls.append(source)
        for name in exporter._GRAPHS:
            (destination / name).write_bytes(name.encode("ascii"))

    monkeypatch.setattr(exporter, "_export", export)

    target = exporter.prepare_models(model_directory())

    assert target == cache / exporter._EXPORT_VERSION
    assert cache.is_dir() and not cache.is_symlink()
    assert not missing_target.parent.exists()
    assert exporter._complete(target)
    assert exporter.prepare_models(model_directory()) == target
    assert calls == [cache]
