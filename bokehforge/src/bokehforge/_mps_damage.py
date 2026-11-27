"""Compute the lightness edge extrema using Metal without lowering precision."""

import numpy as np
import torch
from torch.nn.functional import max_pool2d


def _max_filter(image, size):
    # Reflection for an odd maximum filter only repeats in-bounds pixels.
    # Implicit negative-infinity padding has the same extrema.
    radius = size // 2
    horizontal = max_pool2d(image, (1, size), stride=1, padding=(0, radius))
    return max_pool2d(horizontal, (size, 1), stride=1, padding=(radius, 0))


@torch.inference_mode()
def lightness_extrema(lightness, *, device="mps"):
    image = torch.from_numpy(np.ascontiguousarray(lightness)).to(device)
    # Pool both extrema in one batch.
    upper, neg_lower = _max_filter(torch.stack((image, -image))[None], 3)[0]
    return torch.stack((upper, -neg_lower)).cpu().numpy()


@torch.inference_mode()
def neighborhood_strength(strength, *, device="mps"):
    image = torch.from_numpy(np.ascontiguousarray(strength)).to(device)
    return _max_filter(image[None, None], 9)[0, 0].cpu().numpy()
