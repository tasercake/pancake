"""Command-line entry point for BokehForge."""

import argparse
import sys


def main(argv=None):
    parser = argparse.ArgumentParser(prog="bokehforge", description="A measurement-first workbench for focus stacking.")
    groups = parser.add_subparsers(dest="group", required=True, title="Commands", metavar="COMMAND")
    data = groups.add_parser("data", help="Acquire evaluation datasets")
    actions = data.add_subparsers(dest="action", required=True, title="Data commands", metavar="COMMAND")
    actions.add_parser("download", add_help=False, help="Download verified originals")
    groups.add_parser("bench", add_help=False, help="Score a stacker against RGB ground truth")
    groups.add_parser("inspection", add_help=False, help="Prepare inspection maps for reference photos")
    groups.add_parser("review", add_help=False, help="Browse and rate focus stacks locally")
    args, remaining = parser.parse_known_args(argv)
    if args.group == "bench":
        from .bench import main as command
    elif args.group == "inspection":
        from .inspection import main as command
    elif args.group == "review":
        from .review.server import main as command
    else:
        from .data.download import main as command

    try:
        return command(remaining)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
