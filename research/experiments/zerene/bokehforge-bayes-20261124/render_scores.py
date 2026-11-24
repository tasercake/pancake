"""Render the archived tuning and validation scores."""

import argparse
import json
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    archive = Path(__file__).resolve().parent
    sys.path.insert(0, str(archive / "code"))
    from sweep_bokehforge import plot_scores

    tuning = json.loads((archive / "tuning.json").read_text())
    checkpoints = json.loads((archive / "checkpoints.json").read_text())
    summary = json.loads((archive / "summary.json").read_text())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    plot_scores(tuning, checkpoints, summary["pmax_baseline"], args.output)


if __name__ == "__main__":
    main()
