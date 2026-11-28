"""Local inference for inspection-blend-v1."""

from runpy import run_path
import sys
from unittest.mock import patch

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

from ._workers import workers
from .inspection_assets import ensure_inspection_assets


def _scale(values):
    lo, hi = np.percentile(values, (5, 98))
    return np.clip((values - lo) / max(float(hi - lo), 1e-6), 0, 1).astype(np.float32)


def detail_light(image):
    colors = np.array(image, dtype=np.float32) / 255
    brightness = np.array(image, dtype=np.float32) @ np.float32([.2126, .7152, .0722]) / 255
    scale = max(brightness.shape) / 640
    fine = gaussian_filter(np.abs(brightness - gaussian_filter(brightness, 1.8 * scale)), 6 * scale)
    shading = np.abs(gaussian_filter(brightness, 8 * scale) - gaussian_filter(brightness, 30 * scale))
    saturation = np.ptp(colors, axis=-1)
    contrast = np.abs(gaussian_filter(saturation, 9 * scale) - gaussian_filter(saturation, 36 * scale))
    return .65 * _scale(fine) + .23 * _scale(shading) + .12 * _scale(contrast)


def _reliqs(directory, device, torch):
    weights = directory / "reliqs-weights-ready.pth"
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
    weights = directory / "tempsal-ready.pt"
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
    import torch
    from torchvision.ops import batched_nms
    from transformers import pipeline
    from transformers.models.sam2.image_processing_sam2_fast import _batched_mask_to_box, _compute_stability_score

    folder = directory / "sam2"
    generate = pipeline("mask-generation", model=str(folder), device=device, trust_remote_code=False,
                        model_kwargs={"use_safetensors": True})

    def predict(image):
        # Use the same image embedding, prompts, and prompt batch size as the
        # pipeline. Quality is independent of upsampling and stability, so
        # discard low-IoU masks before expanding them to preview resolution.
        masks, boxes, scores = [], [], []
        sizes = [image.size[::-1]]
        for inputs in generate.preprocess(image, points_per_crop=16, points_per_batch=32):
            inputs.pop("original_sizes")
            inputs.pop("reshaped_input_sizes")
            inputs.pop("input_boxes")  # SAM 2 only sees point prompts on the full image.
            inputs.pop("is_last")
            inputs = generate._ensure_tensor_on_device(inputs, device=generate.device)
            output = generate.model(**inputs)
            quality = output.iou_scores[0].flatten()
            eligible = (quality > .8).nonzero().flatten()
            if eligible.numel() == 0:
                continue
            # Gather from the strided multimask slice without copying masks
            # for points that did not meet the quality threshold.
            batch_masks = output.pred_masks[0]
            per_point = batch_masks.shape[1]
            low = batch_masks[eligible // per_point, eligible % per_point]
            full, = generate.image_processor.post_process_masks([low[None]], sizes, binarize=False)
            full = full[0]
            stable = (_compute_stability_score(full, 0, 1) > .9).nonzero().flatten()
            binary = full.index_select(0, stable) > 0
            masks.append(binary)
            boxes.append(_batched_mask_to_box(binary))
            scores.append(quality[eligible[stable]])
        if not masks:
            return (), ()
        boxes = torch.cat(boxes).cpu().float()
        scores = torch.cat(scores).cpu()
        # The original pipeline suppresses boxes on CPU in prompt order.
        # Keep that tie-breaking and transfer only the selected binary masks.
        keep = batched_nms(boxes, scores, torch.zeros(len(boxes)), .7)
        return torch.cat(masks)[keep.to(device)].cpu().numpy(), scores[keep].numpy()

    return predict


def _mask_heat(masks, scores, detail):
    heat = np.zeros_like(detail)
    for mask, score in zip(masks, scores, strict=True):
        if not .001 <= mask.mean() <= .65:
            continue
        values = detail[mask]
        strength = min(1, float(score)) * (.25 + .75 * float(np.quantile(values, .75)))
        heat[mask] = np.maximum(heat[mask], strength * (.75 + .25 * values))
    return heat


def inspection_components(directory, *, progress=None):
    try:
        import torch
    except ImportError as exc:
        raise RuntimeError("The Inspection blend requires the inspection dependencies") from exc
    if not torch.cuda.is_available():
        raise RuntimeError("The released TempSAL checkpoint requires CUDA")
    ensure_inspection_assets(directory, progress=progress)
    device = "cuda"
    with torch.inference_mode():
        reliqs = _reliqs(directory, device, torch)
        tempsal = _tempsal(directory, device, torch)
        sam2 = _sam2(directory, device)

    @torch.inference_mode()
    def predict(image):
        detail_work = workers().submit(detail_light, image)
        saliency = reliqs(image)
        masks, scores = sam2(image)
        temporal = tempsal(image)
        detail = detail_work.result()
        return {"reliqs": saliency, "detail-light": detail, "sam2": _mask_heat(masks, scores, detail),
                "tempsal": temporal}

    return predict


def blend_components(components):
    maps = np.stack(tuple(components.values()))
    return (1 - np.prod(1 - .5 * maps, axis=0)) / (1 - .5 ** len(maps))


def inspection_blend(directory):
    components = inspection_components(directory)
    return lambda image: blend_components(components(image))
