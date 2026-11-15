"""Command-line entry point for Bokehforge."""

import argparse
import sys


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge", description="A measurement-first workbench for focus stacking.")
    groups = parser.add_subparsers(dest="group", required=True)
    data = groups.add_parser("data", help="Acquire and process evaluation datasets")
    actions = data.add_subparsers(dest="action", required=True)
    actions.add_parser("download", add_help=False, help="Download verified originals within the data budget")
    actions.add_parser("process", add_help=False, help="Convert selected data to canonical linear-sRGB TIFFs")
    args, remaining = parser.parse_known_args(argv)
    try:
        if args.action == "download":
            from .data.download import main as run
        else:
            from .data.process import main as run
        return run(remaining)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
