"""Optimize Zerene DMap on scene-disjoint BokehForge tuning and validation splits."""

import argparse
import json
import random
import statistics
import tempfile
from collections import defaultdict
from hashlib import sha256
from importlib.metadata import version
from pathlib import Path
from types import SimpleNamespace

import matplotlib
import numpy as np
import optuna
import tifffile
from PIL import ImageCms
from run_stack import (
    batch_command,
    batch_lock,
    launch_command,
    make_batch,
    run_zerene,
    write_batches,
)

from bokehforge import SceneCatalog

DATASETS = {"D09": "MFI-WHU", "D29": "InfinigenDefocus"}
STACKS_PER_SPLIT = 50
CHECKPOINT_INTERVAL = 10
CHUNK_SIZE = 10
SEARCH_SPACE = {
    "dmap_percentile": (0, 100),
    "dmap_estimation_radius": (1, 32),
    "dmap_smoothing_radius": (0, 24),
}
DEFAULTS = dict(zip(SEARCH_SPACE, (50, 10, 5)))


def write_json(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)


def select_samples(catalog, seed, count=STACKS_PER_SPLIT):
    rng = random.Random(seed)
    by_dataset = {dataset: defaultdict(list) for dataset in DATASETS}
    for scene_id in sorted(catalog.scene_ids):
        scene = catalog.scene(scene_id)
        if len(scene.reference_ids) == 1 and len(scene.frame_ids) >= 2:
            group = scene.metadata["leakage_group_id"]
            by_dataset[scene.metadata["dataset_id"]][group].append(scene)

    splits = {"tuning": [], "validation": []}
    used_groups = set()
    for dataset, groups in by_dataset.items():
        available = sorted(set(groups) - used_groups)
        if len(available) < 2 * count:
            raise ValueError(f"{dataset} has {len(available)} eligible independent groups, need {2 * count}")
        for index, group in enumerate(rng.sample(available, 2 * count)):
            scene = rng.choice(groups[group])
            split = "tuning" if index < count else "validation"
            splits[split].append({
                "dataset": dataset, "scene_id": scene.id, "group_id": group,
                "source_scene_id": scene.metadata["source_scene_id"],
                "frame_ids": scene.frame_ids, "reference_id": scene.reference_ids[0],
                "reference_semantics": scene.metadata["reference_semantics"],
                "color_recipe_sha256": scene.metadata["recipe_sha256"],
            })
            used_groups.add(group)
    return splits


def prepare_inputs(catalog, splits, directory):
    profile = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
    for split, samples in splits.items():
        print(f"Preparing {len(samples)} {split} stacks", flush=True)
        for sample in samples:
            scene = directory / sample["scene_id"]
            frames = scene / "frames"
            frames.mkdir(parents=True, exist_ok=True)
            reference = scene / "reference.npy"
            data = catalog.decode(sample["reference_id"]).data if not reference.exists() else None
            if data is not None:
                if not np.isfinite(data).all() or data.min() < -1e-6 or data.max() > 1 + 1e-6:
                    raise ValueError(f"Reference is outside the SDR scoring domain: {sample['reference_id']}")
                temporary = reference.with_suffix(".tmp.npy")
                np.save(temporary, data, allow_pickle=False)
                temporary.replace(reference)
            shape = np.load(reference, mmap_mode="r").shape
            for index, frame_id in enumerate(sample["frame_ids"]):
                frame = frames / f"{index:03d}.tif"
                if frame.exists():
                    continue
                pixels = catalog.decode(frame_id).data
                if pixels.shape != shape or not np.isfinite(pixels).all():
                    raise ValueError(f"Unexpected shape or nonfinite pixels: {frame_id}")
                pixels = np.clip(pixels, 0, 1)
                encoded = np.where(pixels <= 0.0031308, 12.92 * pixels, 1.055 * pixels ** (1 / 2.4) - 0.055)
                temporary = frame.with_suffix(".tmp")
                tifffile.imwrite(temporary, np.rint(encoded * 65535).astype(np.uint16),
                                 photometric="rgb", iccprofile=profile, metadata=None)
                temporary.replace(frame)


def score_output(output, reference_path):
    reference = np.load(reference_path, mmap_mode="r")
    with tifffile.TiffFile(output) as source:
        page = source.pages[0]
        pixels = page.asarray()
        if len(source.pages) != 1 or page.photometric != tifffile.PHOTOMETRIC.RGB:
            raise ValueError(f"Expected one RGB image in {output}")
    if pixels.shape != reference.shape or pixels.dtype != np.uint16:
        raise ValueError(f"Expected a registered uint16 RGB TIFF matching the reference: {output}")
    encoded = pixels.astype(np.float32) / 65535
    linear = np.where(encoded <= 0.04045, encoded / 12.92, ((encoded + 0.055) / 1.055) ** 2.4)
    difference = linear - reference
    mse = np.mean(np.square(difference), dtype=np.float64)
    return {
        "psnr_db": float(-10 * np.log10(max(mse, 1e-12))),
        "linear_rgb_mae": float(np.mean(np.abs(difference), dtype=np.float64)),
    }


def evaluate(samples, settings, methods, directory, inputs, java):
    directory.mkdir(parents=True, exist_ok=True)
    record = directory / "settings.json"
    job = {"dmap_parameters": settings, "methods": list(methods), "scenes": [s["scene_id"] for s in samples]}
    if record.exists() and json.loads(record.read_text()) != job:
        raise ValueError(f"Cannot resume with different evaluation settings in {directory}")
    write_json(record, job)

    scores = []
    for start in range(0, len(samples), CHUNK_SIZE):
        chunk = samples[start:start + CHUNK_SIZE]
        result = directory / f"scores-{start:03d}.json"
        if result.exists():
            scores.extend(json.loads(result.read_text()))
            continue
        args = SimpleNamespace(already_aligned=True, **settings)
        for attempt in range(1, 3):
            batch_dir = Path(tempfile.mkdtemp(prefix=f"batch-{start:03d}-", dir=directory))
            try:
                batches = []
                for index, sample in enumerate(chunk):
                    output = batch_dir / f"{index:02d}"
                    output.mkdir()
                    frames = inputs / sample["scene_id"] / "frames"
                    batches.append(make_batch(frames, output, methods, args))
                xml = batch_dir / "batch.xml"
                write_batches(xml, batches)
                run_zerene(batch_command(java, xml), batch_dir / "zerene.log", timeout=900)
                rows, scored_outputs = [], []
                for index, sample in enumerate(chunk):
                    row = {"scene_id": sample["scene_id"], "dataset": sample["dataset"]}
                    reference = inputs / sample["scene_id"] / "reference.npy"
                    for method in methods:
                        matches = list((batch_dir / f"{index:02d}").glob(f"*_{method}.tif"))
                        if len(matches) != 1:
                            raise ValueError(f"Expected one {method} result for {sample['scene_id']}, found {len(matches)}")
                        row[method] = score_output(matches[0], reference)
                        scored_outputs.append(matches[0])
                    rows.append(row)
            except (OSError, ValueError) as error:
                print(f"Batch attempt {attempt} failed in {batch_dir}: {error}", flush=True)
                if attempt == 2:
                    raise
            else:
                write_json(result, rows)
                scores.extend(rows)
                for output in scored_outputs:
                    output.unlink()
                print(f"{directory.name}: scored {len(scores)}/{len(samples)} stacks", flush=True)
                break

    return {
        method: {metric: {
            "all": statistics.fmean(row[method][metric] for row in scores),
            **{dataset: statistics.fmean(row[method][metric] for row in scores if row["dataset"] == dataset)
               for dataset in DATASETS},
        } for metric in ("psnr_db", "linear_rgb_mae")}
        for method in methods
    }


def choose_parameters(tuning, seed, number):
    sampler = optuna.samplers.TPESampler(seed=seed + number, multivariate=True, n_startup_trials=10)
    study = optuna.create_study(sampler=sampler, direction="maximize")
    distributions = {name: optuna.distributions.IntDistribution(*limits) for name, limits in SEARCH_SPACE.items()}
    for row in tuning:
        study.add_trial(optuna.trial.create_trial(
            params=row["parameters"], distributions=distributions, value=row["scores"]["psnr_db"]["all"],
        ))
    seen = {tuple(row["parameters"][name] for name in SEARCH_SPACE) for row in tuning}
    for _ in range(128):
        proposal = study.ask(fixed_distributions=distributions)
        if tuple(proposal.params[name] for name in SEARCH_SPACE) not in seen:
            return proposal.params
        study.tell(proposal, state=optuna.trial.TrialState.FAIL)
    raise ValueError("The optimizer could not propose a previously unevaluated setting")


def plot_scores(tuning, checkpoints, pmax, output):
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, axes = plt.subplots(3, 1, figsize=(9, 9), sharex=True, layout="constrained")
    numbers = [row["trial"] for row in tuning]
    incumbent = tuning[0]
    incumbents = []
    for row in tuning:
        if row["scores"]["psnr_db"]["all"] > incumbent["scores"]["psnr_db"]["all"]:
            incumbent = row
        incumbents.append(incumbent)
    for axis, (dataset, name) in zip(axes, {"all": "Both datasets", **DATASETS}.items()):
        psnr = [row["scores"]["psnr_db"][dataset] for row in tuning]
        axis.scatter(numbers, psnr, label="DMap trial on tuning set", s=13, alpha=0.45, color="#3476a6")
        axis.step(numbers, [row["scores"]["psnr_db"][dataset] for row in incumbents], where="post", color="#19496a",
                  label="Tuning winner (overall PSNR)")
        axis.plot([row["after_trial"] for row in checkpoints],
                  [row["scores"]["psnr_db"][dataset] for row in checkpoints], "o-", color="#d06426",
                  label="Tuning winner on validation set")
        axis.axhline(pmax["validation"]["psnr_db"][dataset], ls="--", color="#777777", lw=1,
                     label="PMax baseline on validation set")
        axis.set_title(name, loc="left")
        axis.set_ylabel("Mean PSNR (dB)")
        axis.grid(axis="y", alpha=0.2)
    axes[0].legend(loc="best", fontsize=8)
    axes[-1].set_xlabel("Optimization trial (0 = default DMap)")
    axes[-1].set_xticks(range(0, tuning[-1]["trial"] + 1, CHECKPOINT_INTERVAL))
    fig.suptitle("Zerene Stacker on BokehForge")
    temporary = output.with_suffix(".tmp.png")
    fig.savefig(temporary, dpi=160)
    temporary.replace(output)
    plt.close(fig)


def run(args):
    out = args.output.resolve()
    out.mkdir(mode=0o700, parents=True, exist_ok=True)
    preferences = Path.home() / "Library/Preferences/ZereneStacker"
    java = launch_command(preferences)
    manifest = out / "experiment.json"
    config = {
        "seed": args.seed, "optimization_trials": args.trials, "stacks_per_dataset_per_split": STACKS_PER_SPLIT,
        "checkpoint_interval": CHECKPOINT_INTERVAL, "search_space": SEARCH_SPACE, "dmap_trial_zero": DEFAULTS,
        "optimizer": "Optuna multivariate TPE with 10 startup observations",
        "alignment": "pre-aligned fixed-coordinate inputs",
        "color": "native resolution, 16-bit tagged sRGB input, Zerene CopyInput output, linear-sRGB scoring",
        "objective": "mean per-stack linear RGB PSNR (unit peak; 120 dB cap), equal weight for each dataset",
        "source_manifests": {dataset: json.loads((args.data / "raw" / dataset / "download-manifest.json").read_text())
                             for dataset in DATASETS},
        "zerene_preferences_sha256": sha256((preferences / "zerenstk.cfg").read_bytes()).hexdigest(),
        "sweep_sha256": sha256(Path(__file__).read_bytes()).hexdigest(),
        "runner_sha256": sha256(Path(__file__).with_name("run_stack.py").read_bytes()).hexdigest(),
        "packages": {name: version(name) for name in ("optuna", "numpy", "tifffile", "bokehforge", "matplotlib")},
    }
    previous = json.loads(manifest.read_text()) if manifest.exists() else None
    if previous and previous["config"] != json.loads(json.dumps(config)):
        raise ValueError(f"Cannot resume with a different experiment configuration in {manifest}")
    inputs = out / "inputs"
    if not (out / "inputs-ready.json").exists():
        print("Verifying and cataloging the pinned native datasets", flush=True)
        catalog = SceneCatalog(args.data, datasets=list(DATASETS))
        splits = select_samples(catalog, args.seed)
        write_json(manifest, {"config": config, "splits": splits})
        prepare_inputs(catalog, splits, inputs)
        write_json(out / "inputs-ready.json", True)
    else:
        splits = previous["splits"]

    tuning, checkpoints, pmax = [], [], {}
    for trial in range(args.trials + 1):
        params = DEFAULTS if trial == 0 else choose_parameters(tuning, args.seed, trial)
        methods = ("dmap", "pmax") if trial == 0 else ("dmap",)
        result = evaluate(splits["tuning"], params, methods, out / "tuning" / f"trial-{trial:03d}", inputs, java)
        tuning.append({"trial": trial, "parameters": params, "scores": result["dmap"]})
        write_json(out / "tuning.json", tuning)
        if trial == 0:
            pmax["tuning"] = result["pmax"]
        if trial % CHECKPOINT_INTERVAL == 0:
            best = max(tuning, key=lambda row: row["scores"]["psnr_db"]["all"])
            result = evaluate(splits["validation"], best["parameters"], methods,
                              out / "validation" / f"after-{trial:03d}", inputs, java)
            if trial == 0:
                pmax["validation"] = result["pmax"]
            entry = {"after_trial": trial, "tuning_winner": best, "scores": result["dmap"]}
            checkpoints.append(entry)
            validation_best = max(checkpoints, key=lambda row: row["scores"]["psnr_db"]["all"])
            entry["best_validation_so_far"] = {
                "after_trial": validation_best["after_trial"], "scores": validation_best["scores"],
                "tuning_winner": validation_best["tuning_winner"],
            }
            write_json(out / "checkpoints.json", checkpoints)
            plot_scores(tuning, checkpoints, pmax, out / "scores.png")
            validation_scores = {"dmap": validation_best["scores"]["psnr_db"]["all"],
                                 "pmax": pmax["validation"]["psnr_db"]["all"]}
            write_json(out / "summary.json", {
                "complete": trial == args.trials, "last_validated_trial": trial,
                "best_tuning_dmap": best, "best_validation_dmap": entry["best_validation_so_far"],
                "pmax_baseline": pmax,
                "best_validation_method": max(validation_scores, key=validation_scores.__getitem__),
            })
            print(f"Checkpoint {trial}: tuning {best['scores']['psnr_db']['all']:.5f} dB, "
                  f"validation {result['dmap']['psnr_db']['all']:.5f} dB", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data", type=Path, default=Path("datasets"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=20261124)
    parser.add_argument("--trials", type=int, default=100, choices=range(0, 101, CHECKPOINT_INTERVAL))
    args = parser.parse_args()
    optuna.logging.set_verbosity(optuna.logging.WARNING)
    preferences = Path.home() / "Library/Preferences/ZereneStacker"
    with batch_lock(preferences):
        run(args)


if __name__ == "__main__":
    main()
