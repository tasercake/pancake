#!/usr/bin/env python3
"""Run the installed macOS Zerene Stacker through its Batch API."""

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

# Load only the shared standard-library adapter so this script needs no Python dependencies.
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "bokehforge/src/bokehforge/review"))
from _zerene import TASK_CODES, batch_command, batch_lock, batch_output, launch_command, write_batch

IMAGE_SUFFIXES = {".jpeg", ".jpg", ".tif", ".tiff"}


def collect_inputs(paths):
    if len(paths) == 1 and paths[0].is_dir():
        paths = [
            path for path in sorted(paths[0].iterdir())
            if not path.name.startswith(".") and path.suffix.lower() in IMAGE_SUFFIXES
        ]
    inputs = [path.expanduser().resolve(strict=True) for path in paths]
    if len(inputs) < 2 or len(set(inputs)) != len(inputs):
        raise ValueError("Provide at least two distinct input frames.")
    for path in inputs:
        if path.suffix.lower() not in IMAGE_SUFFIXES or not path.is_file():
            raise ValueError(f"Expected a TIFF or JPEG input file: {path}")
        if path.stat().st_size == 0:
            raise ValueError(f"Empty input file: {path}")
    return inputs


def run_zerene(command, log_path, timeout):
    with log_path.open("w") as log, subprocess.Popen(
        command, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
        start_new_session=True,
    ) as process:
        try:
            exit_code = process.wait(timeout=timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            if isinstance(error, KeyboardInterrupt):
                raise
            raise ValueError(f"Zerene exceeded {timeout} seconds. See {log_path}.") from error
    if exit_code:
        raise ValueError(f"Zerene exited with status {exit_code}. See {log_path}.")


def collect_outputs(output, methods, log_path):
    results = {}
    for method in methods:
        image = batch_output(output, method)
        info = subprocess.run(
            ["/usr/bin/sips", "-g", "format", "-g", "bitsPerSample", str(image)],
            capture_output=True, text=True, timeout=30,
        )
        properties = set(info.stdout.split())
        if info.returncode or not {"tiff", "16"} <= properties:
            raise ValueError(f"Invalid 16-bit TIFF at {image}. See {log_path}.")
        results[method] = image
    for method, source in results.items():
        destination = output / f"{method}.tif"
        source.rename(destination)
        print(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", type=Path, nargs="+", help="One folder or individual TIFF/JPEG frames")
    parser.add_argument("--output", type=Path, required=True, help="New directory for this run")
    stacking = parser.add_argument_group("Stack settings")
    stacking.add_argument("--method", choices=("both", *TASK_CODES), default="both",
                          help="Choose PMax, DMap, or both (default: both)")
    stacking.add_argument("--already-aligned", action="store_true", help="Use pre-aligned input frames")
    stacking.add_argument("--dmap-percentile", type=float, default=50, help="Fixed contrast percentile (default: 50)")
    stacking.add_argument("--dmap-estimation-radius", type=int, default=10)
    stacking.add_argument("--dmap-smoothing-radius", type=int, default=5)
    limits = parser.add_argument_group("Batch limits")
    limits.add_argument("--timeout", type=int, default=3600, help="Batch timeout in seconds (default: 3600)")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    if not 0 <= args.dmap_percentile <= 100:
        parser.error("--dmap-percentile must be between 0 and 100")
    if args.dmap_estimation_radius < 1 or args.dmap_smoothing_radius < 0:
        parser.error("DMap estimation radius must be positive and smoothing radius nonnegative")

    preferences = Path.home() / "Library/Preferences/ZereneStacker"
    output = args.output.expanduser().resolve()
    log_path = output / "zerene.log"
    methods = tuple(TASK_CODES) if args.method == "both" else (args.method,)
    try:
        inputs = collect_inputs([path.expanduser() for path in args.inputs])
        java = launch_command(preferences)
        with batch_lock(preferences):
            output.mkdir(mode=0o700, parents=True)
            batch_path = output / "batch.xml"
            write_batch(batch_path, output, methods, align=not args.already_aligned,
                        dmap_percentile=args.dmap_percentile, dmap_estimation_radius=args.dmap_estimation_radius,
                        dmap_smoothing_radius=args.dmap_smoothing_radius)
            (output / "inputs.json").write_text(json.dumps([str(path) for path in inputs], indent=2) + "\n")
            command = batch_command(java, batch_path, inputs)
            print(f"Zerene processing {len(inputs)} frames. Log: {log_path}", file=sys.stderr, flush=True)
            started = time.monotonic()
            run_zerene(command, log_path, args.timeout)
            collect_outputs(output, methods, log_path)
            print(f"Finished in {time.monotonic() - started:.1f} seconds.", file=sys.stderr)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Zerene batch failed: {error}\n")
    except KeyboardInterrupt:
        parser.exit(130, f"Zerene batch interrupted. See {log_path}.\n")


if __name__ == "__main__":
    main()
