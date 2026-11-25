"""Run the pinned ShineStacker Python API with its own interpreter."""

import json
from pathlib import Path
import sys

from shinestacker.algorithms import AlignFrames, CombinedActions, DepthMapStack, FocusStack, PyramidStack, StackJob
from shinestacker.config.config import config


def main(folder, output, settings):
    options = json.loads(settings)
    config.init(DISABLE_TQDM=True)
    job = StackJob("review", str(Path.cwd()), input_path=folder)
    if options["align"]:
        job.add_action(CombinedActions("align", [AlignFrames()]))
    algorithm = (PyramidStack(kernel_size=int(options["kernel_size"]))
                 if options["method"] == "pyramid" else DepthMapStack())
    job.add_action(FocusStack("fused", algorithm, naming_mode="TEMPLATE",
                              output_file_template="result", plot_stack=False))
    if not job.run():
        raise RuntimeError("ShineStacker reported a failed job")
    Path("fused/result.tif").replace(output)


if __name__ == "__main__":
    main(*sys.argv[1:])
