"""Convert the pinned checkpoints into a verified local ONNX model cache."""

import argparse
from hashlib import file_digest
from pathlib import Path
from tempfile import TemporaryDirectory

from filelock import FileLock

from .inspection_assets import _prepare_cache, ensure_inspection_assets

_EXPORT_VERSION = "onnx-v2"
_GRAPHS = {"reliqs.onnx", "tempsal.onnx", "sam_encoder.onnx", "sam_decoder.onnx"}
_MANIFEST = "files.sha256"


def _record(path):
    with path.open("rb") as stream:
        return f"{path.name}\t{path.stat().st_size}\t{file_digest(stream, 'sha256').hexdigest()}\n"


def _complete(path):
    try:
        lines = (path / _MANIFEST).read_text(encoding="ascii").splitlines(keepends=True)
        entries = {line.split("\t", 1)[0]: line for line in lines}
        return (len(entries) == len(lines) and _GRAPHS <= entries.keys()
                and all(Path(name).name == name and _record(path / name) == line for name, line in entries.items()))
    except (OSError, ValueError):
        return False


def _export(source, output):
    from ._export_saliency import export_reliqs, export_tempsal
    from .sam_export import export_sam2

    ensure_inspection_assets(source)
    export_reliqs(source, output / "reliqs.onnx")
    export_tempsal(source, output / "tempsal.onnx")
    export_sam2(source, output)


def prepare_models(source):
    source = Path(source).expanduser().absolute()
    target = source / _EXPORT_VERSION
    if _complete(target):
        return target
    _prepare_cache(source)
    with FileLock(source / f".{_EXPORT_VERSION}.lock"):
        if _complete(target):
            return target
        with TemporaryDirectory(prefix=f".{_EXPORT_VERSION}-", dir=source) as staging:
            staging = Path(staging)
            _export(source, staging)
            files = sorted(staging.iterdir())
            if not _GRAPHS <= {path.name for path in files if path.is_file()}:
                raise RuntimeError("The Inspection exporter did not produce every required model")
            manifest = "".join(_record(path) for path in files if path.is_file())
            (staging / _MANIFEST).write_text(manifest, encoding="ascii", newline="\n")
            target.mkdir(exist_ok=True)
            # Commit the checksummed inventory after all graphs and weight files.
            for path in files:
                if path.is_file():
                    path.replace(target / path.name)
            (staging / _MANIFEST).replace(target / _MANIFEST)
    return target


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("version", choices=(_EXPORT_VERSION,))
    args = parser.parse_args(argv)
    prepare_models(args.source)


if __name__ == "__main__":
    main()
