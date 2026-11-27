"""Stacker settings, defaults, and command lines for evaluation runs."""

from dataclasses import asdict, dataclass, replace
import json
from math import isfinite
from pathlib import Path

from . import _zerene
from .stacker_setup import BIN_DIR, EXE, REPO


@dataclass(frozen=True)
class Setting:
    label: str
    default: str | bool | int | float
    choices: dict[str, str] | None = None
    minimum: int | float | None = None
    maximum: int | float | None = None
    when: tuple[str, str] | None = None

    def validate(self, value):
        if self.choices is not None:
            if isinstance(value, str) and value in self.choices:
                return value
            raise ValueError("choose one of the listed values")
        if isinstance(self.default, bool):
            if type(value) is bool:
                return value
            raise ValueError("expected a checkbox value")
        integer = type(self.default) is int
        kinds = (int,) if integer else (int, float)
        if type(value) not in kinds or not self.minimum <= value <= self.maximum or not isfinite(value):
            unit = "a whole number" if integer else "a number"
            raise ValueError(f"enter {unit} from {self.minimum} to {self.maximum}")
        return value if integer else float(value)


AUTO_CPU = {"auto": "Automatic (prefer GPU)", "cpu": "CPU"}
SETTINGS = {
    "pancake": {
        "device": Setting("Processing device", "auto", {**AUTO_CPU, "gpu": "GPU only"}),
        "method": Setting("Fusion method", "hybrid", {
            "hybrid": "Hybrid", "laplacian": "Laplacian pyramid",
            "weighted_average": "Weighted average", "depth_map": "Depth map",
        }),
        "alignment": Setting("Alignment", "off", {
            "off": "Off", "translation": "Translation", "euclidean": "Translation and rotation",
            "similarity": "Translation, rotation and scale", "affine": "Affine", "breathing": "Focus breathing",
        }),
        "crop": Setting("Crop to registered area", False),
    },
    "focus-stack": {
        "device": Setting("Processing device", "auto", {"auto": "Automatic (OpenCL if available)", "cpu": "CPU"}),
        "align": Setting("Align frames", True),
        "keep_size": Setting("Keep full image size", False),
        "consistency": Setting("Consistency filter", 2, minimum=0, maximum=2),
        "denoise": Setting("Wavelet denoise strength", 1.0, minimum=0, maximum=10),
    },
    "openfocus": {
        "device": Setting("Processing device", "auto", AUTO_CPU),
        "method": Setting("Fusion method", "stackmffv3", {
            "stackmffv3": "StackMFF V3", "stackmffv2": "StackMFF V2", "stackmff": "StackMFF V1",
            "openfocus_a": "OpenFocus A", "openfocus_b": "OpenFocus B", "openfocus_c": "OpenFocus C",
            "guided_filter": "Guided filter", "gfgfgf": "GFG-FGF", "dct": "DCT", "dtcwt": "DTCWT",
        }),
        "alignment": Setting("Alignment", "none", {
            "none": "Off", "ecc": "ECC", "homography": "Homography",
            "openfocus": "OpenFocus", "both": "Homography and ECC",
        }),
        "exposure_match": Setting("Match exposure", True),
        "ai_mode": Setting("StackMFF V3 output", "soft", {"soft": "Blend by probability", "hard": "Select sharpest frame"},
                           when=("method", "stackmffv3")),
    },
    "shinestacker": {
        "method": Setting("Fusion method", "pyramid", {"pyramid": "Pyramid", "depth_map": "Depth map"}),
        "align": Setting("Align frames", True),
        "kernel_size": Setting("Pyramid window (pixels)", "5", {"3": "3", "5": "5", "7": "7"},
                               when=("method", "pyramid")),
    },
    "zerene": {
        "method": Setting("Fusion method", "pmax", {"pmax": "PMax", "dmap": "DMap"}),
        "align": Setting("Align frames", True),
        "dmap_percentile": Setting("DMap contrast percentile", 50.0, minimum=0, maximum=100,
                                   when=("method", "dmap")),
        "dmap_estimation_radius": Setting("DMap estimation radius (pixels)", 10, minimum=1, maximum=100,
                                          when=("method", "dmap")),
        "dmap_smoothing_radius": Setting("DMap smoothing radius (pixels)", 5, minimum=0, maximum=100,
                                         when=("method", "dmap")),
    },
}


@dataclass(frozen=True)
class Stacker:
    id: str
    label: str
    method: str
    candidates: tuple[str | Path, ...]
    executable: str | None = None
    options: dict | None = None

    def describe_options(self):
        return {
            name: {**asdict(setting), "step": "any" if type(setting.default) is float else "1"}
            for name, setting in SETTINGS[self.id].items()
        }

    def with_options(self, supplied):
        settings = SETTINGS[self.id]
        if not isinstance(supplied, dict) or supplied.keys() - settings.keys():
            raise ValueError(f"Invalid settings for {self.label}")
        options = {}
        for name, setting in settings.items():
            try:
                options[name] = setting.validate(supplied.get(name, setting.default))
            except ValueError as exc:
                raise ValueError(f"{self.label} · {setting.label}: {exc}") from exc
        method = settings["method"].choices[options["method"]] if "method" in settings else self.method
        return replace(self, method=method, options=options)

    def command(self, frames, output):
        options = self.options if self.options is not None else self.with_options({}).options
        if self.id == "pancake":
            command = [self.executable, "--device", options["device"], "--method", options["method"],
                       "--alignment-mode", options["alignment"], "--bit-depth", "16", "--output", output]
            if options["crop"]:
                command.append("--auto-crop")
            return [*command, "--input", *frames]
        if self.id == "focus-stack":
            command = [self.executable, f"--output={output}", f"--denoise={options['denoise']}",
                       f"--consistency={options['consistency']}"]
            if options["device"] == "cpu":
                command.append("--no-opencl")
            if not options["align"]:
                command.append("--no-align")
            if options["keep_size"]:
                command.append("--nocrop")
            return [*command, *frames]
        if self.id == "openfocus":
            command = [self.executable, "-i", "frames", "-o", output, "--frame-order", "name",
                       "--method", options["method"], "--align", options["alignment"],
                       "--exposure-match" if options["exposure_match"] else "--no-exposure-match"]
            if options["device"] == "cpu":
                command.append("--cpu")
            if options["method"] == "stackmffv3":
                command.extend(("--ai-mode", options["ai_mode"]))
            return command
        if self.id == "shinestacker":
            return [self.executable, str(Path(__file__).with_name("_shine.py")), "frames", output,
                    json.dumps(options)]
        raise ValueError(f"No CLI command for {self.label}")


def configured_stackers(args):
    modules = REPO / ".submodules"
    pancake = "pancake-cli" + EXE
    focus = "focus-stack" + EXE
    openfocus = "openfocus" + EXE
    python = "python" + EXE
    tools = [
        Stacker("pancake", "Pancake Stacker", "Hybrid",
                (args.pancake,) if args.pancake else (REPO / "pancake/target/release" / pancake, pancake)),
        Stacker("focus-stack", "focus-stack", "Complex wavelet",
                (args.focus_stack,) if args.focus_stack else (modules / "focus-stack/build" / focus, focus)),
        Stacker("openfocus", "OpenFocus", "StackMFF V3",
                (args.openfocus,) if args.openfocus else (modules / "OpenFocus/.venv" / BIN_DIR / openfocus, openfocus)),
        Stacker("shinestacker", "ShineStacker", "Pyramid",
                (args.shine_python,) if args.shine_python else (modules / "shinestacker/.venv" / BIN_DIR / python,)),
    ]
    if _zerene.supported():
        tools.append(Stacker("zerene", "Zerene Stacker", "PMax", ()))
    return {tool.id: tool for tool in tools}
