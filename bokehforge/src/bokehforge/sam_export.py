"""Export the pinned SAM 2.1 Inspection checkpoint for native inference."""

from pathlib import Path


def export_sam2(original_models: Path, destination: Path):
    import torch
    from transformers import AutoModelForMaskGeneration

    model = AutoModelForMaskGeneration.from_pretrained(original_models / "sam2", local_files_only=True,
                                                       use_safetensors=True, attn_implementation="sdpa").eval()

    class Encoder(torch.nn.Module):
        def __init__(self):
            super().__init__()
            self.model = model

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
        torch.onnx.export(Encoder().eval(), (torch.zeros(1, 3, 1024, 1024),), destination / "sam_encoder.onnx",
                          input_names=["pixels"], output_names=list(features), opset_version=18, dynamo=False)
        # The three embedding tensors are always on the same model grid.
        embeddings = (torch.zeros(1, 32, 256, 256), torch.zeros(1, 64, 128, 128), torch.zeros(1, 256, 64, 64))
        points = torch.zeros(1, 32, 1, 2)
        torch.onnx.export(Decoder().eval(), (*embeddings, points), destination / "sam_decoder.onnx",
                          input_names=[*features, "points"], output_names=["masks", "quality"],
                          opset_version=18, dynamo=False)
