"""Local inference for inspection-blend-v1. Model assets are supplied separately."""

from hashlib import file_digest
from runpy import run_path
import sys
from unittest.mock import patch

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter


def _verified(path, expected):
    with path.open("rb") as source:
        if file_digest(source, "sha256").hexdigest() != expected:
            raise ValueError(f"Unexpected checkpoint SHA-256: {path}")
    return path


def _scale(values):
    lo, hi = np.percentile(values, (5, 98))
    return np.clip((values - lo) / max(float(hi - lo), 1e-6), 0, 1).astype(np.float32)


def _detail_light(image):
    colors = np.array(image, dtype=np.float32) / 255
    brightness = np.array(image, dtype=np.float32) @ np.float32([.2126, .7152, .0722]) / 255
    scale = max(brightness.shape) / 640
    fine = gaussian_filter(np.abs(brightness - gaussian_filter(brightness, 1.8 * scale)), 6 * scale)
    shading = np.abs(gaussian_filter(brightness, 8 * scale) - gaussian_filter(brightness, 30 * scale))
    saturation = np.ptp(colors, axis=-1)
    contrast = np.abs(gaussian_filter(saturation, 9 * scale) - gaussian_filter(saturation, 36 * scale))
    return .65 * _scale(fine) + .23 * _scale(shading) + .12 * _scale(contrast)


def _reliqs(directory, device, torch):
    weights = _verified(directory / "reliqs-weights-ready.pth",
                        "6ab0acc9c39a616875b01b18a9665abfdab560290955ea27ccd94350d4f6dd83")
    with patch.object(sys, "path", [str(directory / "reliqs"), *sys.path]):
        import tiny_clip
        from patch_sampling.Patch_Sampler import SaliencyPatchSampler, resize_shorter_edge

    checkpoint = torch.load(weights, map_location="cpu", weights_only=True)
    create = tiny_clip.create_model_and_transforms

    def initialize(arch, pretrained, device):
        # The checkpoint includes the whole saliency head, including TinyCLIP.
        return create(arch, pretrained="", device=device)

    with patch.object(tiny_clip, "create_model_and_transforms", initialize):
        sampler = SaliencyPatchSampler(224, 3, 224, "cpu")
    sampler.load_state_dict(checkpoint["patch_sampler_ema"], strict=True)
    sampler = sampler.eval().to(device)

    def predict(image):
        # Bound the working grid for extreme aspect ratios. Both grids cover the full scene.
        working = image.resize(tuple(max(64, side) for side in image.size), Image.Resampling.BILINEAR)
        pixels = torch.from_numpy(np.array(working)).permute(2, 0, 1).to(device, torch.float32) / 255
        if min(working.size) < 224:
            pixels = resize_shorter_edge(pixels, 224, mode="bilinear")
        pixels = resize_shorter_edge(pixels, 224, mode="area")[None]
        h, w = pixels.shape[-2:]
        pad_w, pad_h = (-w) % 16, (-h) % 16
        padded = torch.nn.functional.pad(pixels, (0, pad_w, 0, pad_h), mode="reflect")
        features = sampler.saliency_model(sampler.image_preprocess(padded), dense_features=True)
        grid = sampler.ln_post(features).reshape(1, (h + pad_h) // 16, (w + pad_w) // 16, -1)
        logits = sampler.projection(grid.permute(0, 3, 1, 2))[0, 0, :h, :w]
        density = torch.softmax(logits.flatten() / sampler.log_T.exp(), dim=0).reshape(h, w)
        result = torch.nn.functional.interpolate(density[None, None], (image.height, image.width),
                                                 mode="bilinear", align_corners=False)[0, 0]
        return (result / (result.max() + 1e-8)).float().cpu().numpy()

    return predict


def _tempsal(directory, device, torch):
    weights = _verified(directory / "tempsal-ready.pt",
                        "04d9d80f77ed2fcd7b4461440f5715a8a78e418f83bc7adbc658a991e04be9e9")
    code = directory / "tempsal-code"
    with patch.object(sys, "path", [str(code), str(code / "PNAS"), *sys.path]):
        constructor = run_path(str(code / "model.py"))["PNASBoostedModelMultiLevel"]
    model = constructor(torch.device(device), str(weights), str(weights), 5, train_model=0).eval().to(device)

    def predict(image):
        pixels = np.array(image.resize((256, 256), Image.Resampling.BILINEAR), dtype=np.float32) / 127.5 - 1
        tensor = torch.from_numpy(pixels).permute(2, 0, 1)[None].to(device)
        overall, _ = model(tensor)
        overall = overall.reshape(256, 256).float().cpu().numpy()
        overall /= max(float(overall.max()), 1e-8)
        return np.asarray(Image.fromarray(overall).resize(image.size, Image.Resampling.BILINEAR))

    return predict


def _sam2(directory, device):
    from transformers import pipeline

    folder = directory / "sam2"
    _verified(folder / "model.safetensors", "dc407dce21301fd94abb395c5099b4f2c455fdc8a8f261ac3d0ea6d4cd197230")
    generate = pipeline("mask-generation", model=str(folder), device=device, trust_remote_code=False,
                        model_kwargs={"use_safetensors": True})

    def predict(image, detail):
        data = generate(image, points_per_crop=16, points_per_batch=32,
                        pred_iou_thresh=.8, stability_score_thresh=.9)
        heat = np.zeros_like(detail)
        for mask, score in zip(data["masks"], data["scores"], strict=True):
            mask = np.asarray(mask, dtype=bool)
            if not .001 <= mask.mean() <= .65:
                continue
            strength = min(1, float(score)) * (.25 + .75 * float(np.quantile(detail[mask], .75)))
            heat[mask] = np.maximum(heat[mask], strength * (.75 + .25 * detail[mask]))
        return heat

    return predict


def inspection_blend(directory):
    try:
        import torch
    except ImportError as exc:
        raise RuntimeError("Preparing inspection maps requires the optional CUDA model environment") from exc
    if not torch.cuda.is_available():
        raise RuntimeError("Preparing inspection maps with the released TempSAL checkpoint requires CUDA")
    device = "cuda"
    with torch.inference_mode():
        reliqs = _reliqs(directory, device, torch)
        tempsal = _tempsal(directory, device, torch)
        sam2 = _sam2(directory, device)

    @torch.inference_mode()
    def predict(image):
        detail = _detail_light(image)
        maps = np.stack([reliqs(image), detail, sam2(image, detail), tempsal(image)])
        return (1 - np.prod(1 - .5 * maps, axis=0)) / (1 - .5 ** len(maps))

    return predict
