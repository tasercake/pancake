"""Export the verified ReLIQS and TempSAL checkpoints for native inference."""

import sys
from pathlib import Path
from unittest.mock import patch


def _reliqs_model(assets: Path):
    import torch

    code = assets / "reliqs"
    with patch.object(sys, "path", [str(code), *sys.path]):
        import tiny_clip
        from patch_sampling.Patch_Sampler import SaliencyPatchSampler

    create = tiny_clip.create_model_and_transforms

    def initialize(arch, pretrained, device):
        # The checkpoint includes TinyCLIP and the saliency decoder.
        return create(arch, pretrained="", device=device)

    with patch.object(tiny_clip, "create_model_and_transforms", initialize):
        sampler = SaliencyPatchSampler(224, 3, 224, "cpu")
    weights = torch.load(assets / "reliqs-weights-ready.pth", map_location="cpu", weights_only=True, mmap=True)
    sampler.load_state_dict(weights["patch_sampler_ema"], strict=True)

    class ReLIQS(torch.nn.Module):
        def __init__(self, sampler):
            super().__init__()
            self.sampler = sampler

        def forward(self, pixels):
            sampler = self.sampler
            features = sampler.saliency_model(sampler.image_preprocess(pixels), dense_features=True)
            height, width = pixels.shape[-2:]
            grid = sampler.ln_post(features).reshape(1, height // 16, width // 16, -1)
            return sampler.projection(grid.permute(0, 3, 1, 2)) / sampler.log_T.exp()

    return ReLIQS(sampler).eval()


def _tempsal_model(assets: Path):
    import torch

    weights = assets / "tempsal-ready.pt"
    code = assets / "tempsal-code"
    path = code / "model.py"
    source = path.read_text(encoding="utf-8")
    # Apply only these edits to the verified upstream source, in memory.
    # Its checkpoint keys reflect DataParallel, which is unnecessary for inference.
    edits = {
        "nn.DataParallel(": ("CheckpointPrefix(", 4),
        ".cuda()": ("", 3),
        "torch.load(model_path)": (
            'torch.load(model_path, map_location="cpu", weights_only=True, mmap=True)', 1,
        ),
    }
    for original, (replacement, count) in edits.items():
        if source.count(original) != count:
            raise RuntimeError(f"Unexpected TempSAL source at {path}")
        source = source.replace(original, replacement)

    class CheckpointPrefix(torch.nn.Module):
        def __init__(self, module):
            super().__init__()
            self.module = module

    namespace = {"__name__": "_tempsal_export", "__file__": str(path), "CheckpointPrefix": CheckpointPrefix}
    with patch.object(sys, "path", [str(code), str(code / "PNAS"), *sys.path]):
        exec(compile(source, str(path), "exec"), namespace)
        model = namespace["PNASBoostedModelMultiLevel"](torch.device("cpu"), str(weights), str(weights), 5,
                                                        train_model=0)
    for name in ("pnas_sal", "pnas_vol"):
        branch = getattr(model, name)
        while isinstance(branch, CheckpointPrefix):
            branch = branch.module
        setattr(model, name, branch)

    class TempSAL(torch.nn.Module):
        def __init__(self, model):
            super().__init__()
            self.model = model

        def forward(self, pixels):
            overall, _ = self.model(pixels)
            return overall

    return TempSAL(model).eval()


def export_reliqs(assets: Path, output: Path):
    import torch

    with torch.no_grad():
        model = _reliqs_model(assets)
        # TinyCLIP's equivalent explicit attention keeps the token axis dynamic.
        # The TorchScript exporter otherwise specializes MultiheadAttention's reshape.
        for block in model.sampler.saliency_model.transformer.resblocks:
            block.attn.use_naive_compute = True
        pixels = torch.zeros((1, 3, 224, 320))
        torch.onnx.export(
            model, pixels, str(output), opset_version=18, dynamo=False,
            input_names=["pixels"], output_names=["logits"],
            dynamic_axes={"pixels": {2: "height", 3: "width"}, "logits": {2: "height", 3: "width"}},
        )


def export_tempsal(assets: Path, output: Path):
    import torch

    with torch.no_grad():
        model = _tempsal_model(assets)
        torch.onnx.export(
            model, torch.zeros((1, 3, 256, 256)), str(output),
            opset_version=18, dynamo=False, input_names=["pixels"], output_names=["saliency"],
        )
