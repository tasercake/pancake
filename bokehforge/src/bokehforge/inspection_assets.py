"""Pinned upstream assets for the Inspection blend."""

from pathlib import Path
import sys

from filelock import FileLock

from .data.download import acquire, session, verified


_RELIQS_CODE = "https://raw.githubusercontent.com/hakan-emre-gedik/ReLIQS/6dd687552cb471c6b834236d917a3c3e43a28d4d"
_RELIQS_WEIGHTS = "https://huggingface.co/hakanemre/ReLIQS/resolve/48761756eb3ca6d2386e17780cf2faa2622a8742"
_TEMPSAL_CODE = "https://raw.githubusercontent.com/IVRL/Tempsal/4bdf3d5f173f390da0f714dfd10873c701d7ca32/src"
_TEMPSAL_WEIGHTS = "https://drive.usercontent.google.com/download?id=1LzZJal2BMjFcOpwI9uOsOC0IQcZVzmEP&export=download&confirm=t"
_SAM2 = "https://huggingface.co/facebook/sam2.1-hiera-large/resolve/665f8e2ad61cf5f53d65644ff27c8ee525124610"


def _asset(path, url, size, sha256):
    return {"path": path, "url": url, "size": size, "sha256": sha256}


def _bundle(folder, source, files):
    return tuple(_asset(f"{folder}/{name}", f"{source}/{name}", size, checksum)
                 for name, size, checksum in files)


_RELIQS = _bundle("reliqs", _RELIQS_CODE, (
    ("patch_sampling/Patch_Sampler.py", 11264, "3a24f79b442e8d18f4049701c33273e254b8e0774b27e8b73316e1ec9bfb6239"),
    ("utils/utils.py", 3576, "f76c9d006271eae7c0d3531b1c5cbde984bb8723c6c6c2cebf04b2bd01584294"),
    ("tiny_clip/bpe_simple_vocab_16e6.txt.gz", 1356917, "924691ac288e54409236115652ad4aa250f48203de50a9e4722a6ecd48d6804a"),
    ("tiny_clip/model_configs/TinyCLIP-ViT-8M-16-Text-3M.json", 294, "9ca063ed7d11827ec00ed8f07cf8754e864e23145db8ca6f2544acc0327b2c63"),
    ("tiny_clip/__init__.py", 705, "8433761637832e32e98e0525a81e538fd2d20993462be3310ba3a6e320bd569f"),
    ("tiny_clip/constants.py", 116, "4289fac298908e2fb814ff84b4594ff49094a7eab074fed1b2ca6429339e923b"),
    ("tiny_clip/factory.py", 8621, "78aedaaf84072b8e263910e158b8802ceb95559ff432a672d5fa53c163f51b2f"),
    ("tiny_clip/l0module.py", 15505, "63fe0484faa961a7e1edf12c880ae509c4a37e85675c41ee397278684d9bd8e0"),
    ("tiny_clip/loss.py", 6496, "57df9e1d85fcdbce3e50d4a7d9f0a828a4c69c261395ed3bf455eeceb06ee53b"),
    ("tiny_clip/model.py", 55401, "c14744d867696f7501d20fbd545a5e70017f92f78f78b71dc33640a4980bd97d"),
    ("tiny_clip/openai.py", 4769, "035fb86bf84fd08c84bf23f1ae78e7b2a1bf7dd7abc7d2c743bafec39ecbaf19"),
    ("tiny_clip/pretrained.py", 13525, "2553cc62e09a652a8c5558edda35fcb1086996e5b8d89d3da7775fe672d1b491"),
    ("tiny_clip/resnet.py", 7178, "db3bac83eb9a35ab81e77b80f909ce297b1a3c6afea680327eec258fa6eef61a"),
    ("tiny_clip/timm_model.py", 4596, "a8327aad10a43f1b9f574ba2ad8880233b35e0a272d3ac4992a2c4bbc24311c9"),
    ("tiny_clip/tokenizer.py", 7425, "3585649c38207cf303697135f7e898cf115a63d71227f2d76f09fe6decc0ea1c"),
    ("tiny_clip/transform.py", 4224, "f0120c4d9ad1a24c8d1b056f37cf3e238b369d6a3791ee8adf284628bece7837"),
    ("tiny_clip/utils.py", 2260, "e87b6b451ed5e69c2118520f4be59eb8c3662097db7ea009314249446db04992"),
    ("tiny_clip/version.py", 22, "92e9a21889a1cce4e54eb44cfba8c3a3698d9d51c63bfdafc6dae10749f3022c"),
))

_TEMPSAL = _bundle("tempsal-code", _TEMPSAL_CODE, (
    ("model.py", 11457, "e7e67330f462516dd6b4dd01a9464682c0c2d823c04eb9f051051ccd9a7ee3d9"),
    ("PNAS/PNASnet.py", 4497, "7c1115a585c26a1891ce9ca39fa777e490a6390087d1888c2481c8f06545bd57"),
    ("PNAS/genotypes.py", 745, "991926f5535aae4c69f1cd7ad54e271b6754806f26e547f39a96a49816c7bd9e"),
    ("PNAS/operations.py", 4454, "89c078173e13d4bdcc9bf06115df910ba24c4753aea184496b9e7505315d54b6"),
))

_SAM2_CONFIG = _bundle("sam2", _SAM2, (
    ("config.json", 5705, "00446988cf4d617118d2d347eabe2c46aebed744628facdd540508be30b69ec3"),
    ("preprocessor_config.json", 683, "6ebf229ee259368ce4a8d4f2fe893a72b053023710853e257253939e601f583d"),
    ("processor_config.json", 95, "f8a68e865cfad115c1c2763f3d93eca7b1c622da06da2a9273eb437fb2389b6d"),
    ("video_preprocessor_config.json", 705, "9fccfe5f464ec38c2f236d0e6a68e95511c80c22132fc2fa4b9f7b65f24fad95"),
))


_ASSETS = (
    *_RELIQS,
    _asset("reliqs-weights-ready.pth", f"{_RELIQS_WEIGHTS}/KONIQ/model_weights.pth?download=true", 1280959843,
           "6ab0acc9c39a616875b01b18a9665abfdab560290955ea27ccd94350d4f6dd83"),
    *_TEMPSAL,
    _asset("tempsal-ready.pt", _TEMPSAL_WEIGHTS, 972654723,
           "04d9d80f77ed2fcd7b4461440f5715a8a78e418f83bc7adbc658a991e04be9e9"),
    *_SAM2_CONFIG,
    _asset("sam2/model.safetensors", f"{_SAM2}/model.safetensors?download=true", 897897416,
           "dc407dce21301fd94abb395c5099b4f2c455fdc8a8f261ac3d0ea6d4cd197230"),
)


def ensure_inspection_assets(directory: Path):
    # Inspect the known paths first so a complete, even read-only, cache is usable offline.
    pending = [asset for asset in _ASSETS if not verified(directory / asset["path"], asset.copy())]
    if not pending:
        return

    directory.mkdir(parents=True, exist_ok=True)
    # The dataset downloader uses a predictable .part path so a later run can resume.
    # Serialize writers to this cache across Bench and Review processes.
    with FileLock(directory / ".download.lock"), session() as client:
        for asset in pending:
            target = directory / asset["path"]
            print(f"Preparing BokehForge inspection asset {target.name}", file=sys.stderr, flush=True)
            try:
                acquire(client, target, asset.copy(), reserve=0, retries=3)
            except (OSError, RuntimeError, ValueError) as exc:
                raise RuntimeError(f"Could not cache inspection asset {asset['path']}: {exc}") from exc
