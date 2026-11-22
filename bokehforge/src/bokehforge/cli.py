"""Command-line entry point for Bokehforge."""

import argparse
import sys


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge", description="A measurement-first workbench for focus stacking.")
    groups = parser.add_subparsers(dest="group", required=True)
    data = groups.add_parser("data", help="Acquire evaluation datasets")
    actions = data.add_subparsers(dest="action", required=True)
    actions.add_parser("download", add_help=False, help="Download verified originals")
    _, remaining = parser.parse_known_args(argv)
    from .data.download import main as download

    try:
        return download(remaining)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
