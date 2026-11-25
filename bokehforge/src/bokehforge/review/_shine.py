"""Run the pinned ShineStacker Python API with its own interpreter."""

from pathlib import Path
import sys

from shinestacker.algorithms import AlignFrames, CombinedActions, FocusStack, PyramidStack, StackJob
from shinestacker.config.config import config


def main(folder, output):
    config.init(DISABLE_TQDM=True)
    job = StackJob("review", str(Path.cwd()), input_path=folder)
    job.add_action(CombinedActions("align", [AlignFrames()]))
    job.add_action(FocusStack("fused", PyramidStack(), naming_mode="TEMPLATE",
                              output_file_template="result", plot_stack=False))
    if not job.run():
        raise RuntimeError("ShineStacker reported a failed job")
    Path("fused/result.tif").replace(output)


if __name__ == "__main__":
    main(*sys.argv[1:])
