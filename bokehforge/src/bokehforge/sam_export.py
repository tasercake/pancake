"""Export the pinned SAM 2.1 Inspection checkpoint for native inference."""

from itertools import groupby
from pathlib import Path
from unittest.mock import patch


def _partition_unpadded(states, window):
    batch, height, width, channels = states.shape
    if height % window or width % window:
        raise ValueError("The fixed SAM encoder grid must divide into whole attention windows")
    if height == width == window:
        return states, (height, width)
    tiles = states.reshape(batch, height // window, window, width // window, window, channels)
    tiles = tiles.permute(0, 1, 3, 2, 4, 5).reshape(-1, window, window, channels)
    return tiles, (height, width)


def _unpartition_unpadded(tiles, window, padded, size):
    if padded != size:
        raise ValueError("The fixed SAM encoder grid must use unpadded attention windows")
    height, width = size
    if height == width == window:
        return tiles
    channels = tiles.shape[-1]
    tiles = tiles.reshape(-1, height // window, width // window, window, window, channels)
    return tiles.permute(0, 1, 3, 2, 4, 5).reshape(-1, height, width, channels)


def _windowed_forward(backbone, output_type):
    def local_window(block):
        if block.query_stride is None and block.dim == block.dim_out:
            return block.window_size
        return 0

    def forward(pixel_values, **kwargs):
        states = backbone.patch_embed(pixel_values)
        states = states + backbone._get_pos_embed(states.shape[1:3])
        features = []
        start = 0
        for end in backbone.stage_ends:
            # Within a stage, adjacent local blocks share the same window
            # layout. Keep those tokens together until the next grid consumer.
            for window, blocks in groupby(backbone.blocks[start:end + 1], key=local_window):
                if window:
                    states, size = _partition_unpadded(states, window)
                for block in blocks:
                    states = block(states, **kwargs)
                if window:
                    states = _unpartition_unpadded(states, window, size, size)
            features.append(states)
            start = end + 1
        return output_type(last_hidden_state=states, intermediate_hidden_states=tuple(features))

    return forward


def _validate_encoder(path):
    import onnx

    onnx.checker.check_model(path, full_check=True)
    model = onnx.load_model(path, load_external_data=False)

    def contract(entries):
        return [
            (entry.name, entry.type.tensor_type.elem_type,
             tuple(dim.dim_value for dim in entry.type.tensor_type.shape.dim))
            for entry in entries
        ]

    f32 = onnx.TensorProto.FLOAT
    if ([(entry.domain, entry.version) for entry in model.opset_import] != [("", 18)]
            or any(node.domain for node in model.graph.node)
            or contract(model.graph.input) != [("pixels", f32, (1, 3, 1024, 1024))]
            or contract(model.graph.output) != [
                ("high", f32, (1, 32, 256, 256)), ("middle", f32, (1, 64, 128, 128)),
                ("low", f32, (1, 256, 64, 64)),
            ]):
        raise RuntimeError("The exported SAM encoder has an unexpected ONNX operator or tensor contract")


def export_sam2(original_models: Path, destination: Path):
    import torch
    from transformers import AutoModelForMaskGeneration
    from transformers.models.sam2 import modeling_sam2

    model = AutoModelForMaskGeneration.from_pretrained(original_models / "sam2", local_files_only=True,
                                                       use_safetensors=True, attn_implementation="sdpa").eval()

    class Encoder(torch.nn.Module):
        def __init__(self, sample):
            super().__init__()
            self.model = model
            backbone = model.vision_encoder.backbone
            shape = backbone.patch_embed(sample).shape[1:3]
            self.register_buffer("position", backbone._get_pos_embed(shape).contiguous())

        def forward(self, pixels):
            return tuple(self.model.get_image_embeddings(pixels))

    class Decoder(torch.nn.Module):
        def __init__(self):
            super().__init__()
            self.model = model

        def forward(self, high, middle, low, points):
            result = self.model(image_embeddings=[high, middle, low], input_points=points,
                                input_labels=torch.ones_like(points[..., 0], dtype=torch.int64))
            return result.pred_masks, result.iou_scores

    features = ("high", "middle", "low")
    with torch.no_grad():
        sample = torch.zeros(1, 3, 1024, 1024)
        encoder = Encoder(sample).eval()
        # The encoder always receives this grid size. Export its learned positional
        # embedding as one initializer so ONNX Runtime reads the fully tiled grid.
        backbone = model.vision_encoder.backbone
        forward = _windowed_forward(backbone, modeling_sam2.Sam2HieraDetModelOutput)
        with (
            patch.object(backbone, "forward", forward),
            patch.object(backbone, "_get_pos_embed", lambda _shape: encoder.position),
            patch.object(modeling_sam2, "window_partition", _partition_unpadded),
            patch.object(modeling_sam2, "window_unpartition", _unpartition_unpadded),
        ):
            torch.onnx.export(encoder, (sample,), destination / "sam_encoder.onnx",
                              input_names=["pixels"], output_names=list(features), opset_version=18, dynamo=False)
        # The three embedding tensors are always on the same model grid.
        embeddings = (torch.zeros(1, 32, 256, 256), torch.zeros(1, 64, 128, 128), torch.zeros(1, 256, 64, 64))
        points = torch.zeros(1, 32, 1, 2)
        torch.onnx.export(Decoder().eval(), (*embeddings, points), destination / "sam_decoder.onnx",
                          input_names=[*features, "points"], output_names=["masks", "quality"],
                          opset_version=18, dynamo=False)
