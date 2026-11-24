#!/usr/bin/env python3
"""Run the installed macOS Zerene Stacker through its Batch API."""

import argparse
import fcntl
import json
import os
import shlex
import signal
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
from contextlib import contextmanager
from pathlib import Path

MAIN_CLASS = "com.zerenesystems.stacker.gui.MainFrame"
IMAGE_SUFFIXES = {".jpeg", ".jpg", ".tif", ".tiff"}
TASK_CODES = {"pmax": "1", "dmap": "2"}


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


def launch_command(preferences):
    recorded = preferences / "zerenstk.launchcmd"
    if not recorded.is_file():
        raise ValueError(f"Open Zerene Stacker once to create {recorded}.")
    command = shlex.split(recorded.read_text())
    if MAIN_CLASS not in command or not Path(command[0]).is_file():
        raise ValueError(f"Invalid launch command in {recorded}. Start Zerene to refresh it.")
    # The recorded launch can include Lightroom's temporary inputs and flags.
    return command[:command.index(MAIN_CLASS) + 1]


def make_batch(source, output, methods, args):
    batch = ET.Element("Batch")
    sources = ET.SubElement(batch, "Sources", length="1")
    ET.SubElement(sources, "Source", value=str(source))
    ET.SubElement(batch, "ProjectDispositionCode", value="101")  # Temporary project
    tasks = ET.SubElement(batch, "Tasks", length=str(len(methods)))
    for method in methods:
        task = ET.SubElement(tasks, "Task")
        ET.SubElement(task, "OutputImageDispositionCode", value="4")  # Designated folder
        ET.SubElement(task, "OutputImagesDesignatedFolder", value=str(output))
        preferences = ET.SubElement(task, "Preferences")
        values = {
            "OutputImageNaming.Template": method,
            "AlignmentControl.AddNewFilesAsAlreadyAligned": str(args.already_aligned).lower(),
            "SaveImage.FileType": "tif",
            "SaveImage.BitsPerColor": "16",
            "UIStyle.ShowIntermediateOutputInterval": "0.0",
        }
        if method == "pmax":
            values.update({
                "PyramidControl.RetainUDRImage": "false",
                "PyramidControl.RetainOnlyUDRImage": "false",
            })
        else:
            values.update({
                "DepthMapControl.UseFixedContrastThresholdLevel": "false",
                "DepthMapControl.UseFixedContrastThresholdPercentile": "true",
                "DepthMapControl.ContrastThresholdPercentile": str(args.dmap_percentile),
                "DepthMapControl.EstimationRadius": str(args.dmap_estimation_radius),
                "DepthMapControl.SmoothingRadius": str(args.dmap_smoothing_radius),
                "DepthMapControl.SaveDepthMapImage": "false",
            })
        for key, value in values.items():
            ET.SubElement(preferences, key, value=value)
        ET.SubElement(task, "TaskIndicatorCode", value=TASK_CODES[method])
    return batch


def write_batches(path, batches):
    root = ET.Element("ZereneStackerBatchScript")
    queue = ET.SubElement(root, "BatchQueue")
    ET.SubElement(queue, "Batches", length=str(len(batches))).extend(batches)
    ET.indent(root)
    ET.ElementTree(root).write(path, encoding="UTF-8", xml_declaration=True)


def write_batch(path, output, methods, args):
    write_batches(path, [make_batch("%CurrentProject%", output, methods, args)])


def batch_command(java, batch_path, inputs=()):
    return [
        # Set the macOS activation policy before AWT initializes.
        *java[:-1], "-Dapple.awt.BackgroundOnly=true", java[-1],
        "-noSplashScreen", "-runMinimized", "-showProgressWhenMinimized=false",
        "-suppressSaveProperties", "-exitOnBatchScriptCompletion",
        "-batchScript", str(batch_path), *map(str, inputs),
    ]


@contextmanager
def batch_lock(preferences):
    with (preferences / "batch-runner.lock").open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError("Another Zerene batch runner is active.") from error
        yield


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
        files = sorted(output.glob(f"*_{method}.tif"))
        if len(files) != 1:
            raise ValueError(f"Expected one {method} TIFF, found {len(files)}. See {log_path}.")
        info = subprocess.run(
            ["/usr/bin/sips", "-g", "format", "-g", "bitsPerSample", str(files[0])],
            capture_output=True, text=True, timeout=30,
        )
        properties = set(info.stdout.split())
        if info.returncode or not {"tiff", "16"} <= properties:
            raise ValueError(f"Invalid 16-bit TIFF at {files[0]}. See {log_path}.")
        results[method] = files[0]
    for method, source in results.items():
        destination = output / f"{method}.tif"
        source.rename(destination)
        print(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", type=Path, nargs="+", help="One folder or individual TIFF/JPEG frames")
    parser.add_argument("--output", type=Path, required=True, help="New directory for this run")
    parser.add_argument("--method", choices=("both", *TASK_CODES), default="both")
    parser.add_argument("--timeout", type=int, default=3600, help="Batch timeout in seconds (default: 3600)")
    parser.add_argument("--already-aligned", action="store_true", help="Use pre-aligned input frames")
    parser.add_argument("--dmap-percentile", type=float, default=50, help="Fixed contrast percentile (default: 50)")
    parser.add_argument("--dmap-estimation-radius", type=int, default=10)
    parser.add_argument("--dmap-smoothing-radius", type=int, default=5)
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
            write_batch(batch_path, output, methods, args)
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
