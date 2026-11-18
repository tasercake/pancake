"""Frozen profile selection, independent TIFF checks, and sample accounting."""

import hashlib
import json
import math
import statistics
from dataclasses import dataclass

import numpy as np
import tifffile


def sha256(path):
    """Stream original bytes without building an in-memory corpus."""
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def pinned_json(path, digest):
    """Load one immutable input specification after checking its recorded hash."""
    if sha256(path) != digest:
        raise ValueError(f"SHA-256 mismatch for frozen specification: {path}")
    return json.loads(path.read_text())


@dataclass(frozen=True)
class Profile:
    """One frozen workload, method, and explicit device preference."""

    workload: str
    method: str
    device: str
    options: dict

    @property
    def key(self):
        return f"{self.workload}.{self.method}.{self.device}"


@dataclass(frozen=True)
class Campaign:
    """p0-initial-1 measurements and thresholds, loaded from the owning document."""

    raw: dict

    @classmethod
    def load(cls, path):
        raw = json.loads(path.read_text())
        if raw["schema_version"] != 1 or raw["targets_version"] != "p0-initial-1":
            raise ValueError("unsupported performance target specification")
        return cls(raw)

    def profiles(self, device):
        if device not in ("cpu", "gpu"):
            raise ValueError(f"explicit cpu or gpu device required, got {device}")
        common = self.raw["common_profile"]
        for workload in self.raw["workloads"]:
            for method in workload["methods"]:
                yield Profile(
                    workload["id"],
                    method,
                    device,
                    {
                        "device": device,
                        "method": method,
                        "align": True,
                        "alignment_mode": workload["alignment_mode"],
                        "alignment_reference": common["alignment_reference"],
                        "kernel_size": common["kernel_size"],
                        "pyramid_levels": common["pyramid_levels"],
                        "scale_factor": common["scale_factor"],
                        "slab_size": workload.get("slab_size", 0),
                        "slab_overlap": workload.get("slab_overlap", 0),
                        "bit_depth": common["output_bit_depth"],
                        "auto": common["auto_parameters"],
                        "auto_crop": common["auto_crop"],
                        "quality_report": common["quality_report"],
                    },
                )

    def schedule(self):
        protocol = self.raw["measurement_protocol"]
        warm = protocol["warm"]
        counts = (
            ("warmup", warm["warmup_complete_jobs"]),
            ("warm", warm["measured_complete_jobs"]),
            ("cold", protocol["cold"]["measured_complete_jobs"]),
        )
        jobs = []
        reverse = False
        for kind, count in counts:
            for number in range(1, count + 1):
                order = (
                    ("candidate", "reference")
                    if reverse
                    else ("reference", "candidate")
                )
                jobs.extend((kind, number, target) for target in order)
                reverse = not reverse
        return jobs


def corpus(repo, campaign, workload):
    """Resolve ordered file records and photograph/recipe provenance from P0 pins."""
    evidence = campaign.raw["corpus_evidence"]
    if workload == "T50":
        pin = evidence["T50"]
        path = repo / pin["receipt_path"]
        frozen = pinned_json(path, pin["receipt_sha256"])
        records = [
            {**item, "path": item["name"], "sha256": item["file_sha256"]}
            for item in frozen["files"]
            if item["focus"] is not None
        ]
        provenance = {
            key: frozen[key]
            for key in (
                "recipe",
                "sources",
                "versions",
                "known_truth",
                "generated_pixels_license",
                "conversion",
            )
        }
    elif workload == "R50":
        pin = evidence["R50"]
        path = repo / pin["manifest_path"]
        frozen = pinned_json(path, pin["manifest_sha256"])
        records = frozen["frames"]
        provenance = {
            "source": frozen["source"],
            "full_raw_decode": frozen["qualification"],
        }
    else:
        path = repo / evidence["manifest_path"]
        frozen = pinned_json(path, evidence["manifest_sha256"])
        selected = next(
            item
            for item in frozen["workloads"]
            if item["id"] == evidence[workload]["selected_id"]
        )
        records = selected["frames"]
        provenance = {
            "source": next(
                item
                for item in frozen["sources"]
                if item["id"] == selected["dataset_id"]
            ),
            "encoding": selected["encoding"],
        }
    provenance.update({"path": str(path), "sha256": sha256(path)})
    return records, provenance


def verify_frames(root, records, count, *, distinct_content=True):
    """Require a complete exact-byte set under the explicit staging root."""
    root = root.resolve()
    if len(records) != count:
        raise ValueError(f"expected {count} ordered frames")
    if distinct_content and len({record["sha256"] for record in records}) != count:
        raise ValueError(f"expected {count} distinct ordered frames")
    ordered = []
    for record in records:
        path = (root / record["path"]).resolve(strict=True)
        if not path.is_relative_to(root) or not path.is_file():
            raise ValueError(f"image is not a regular file inside staging root: {path}")
        size = record.get("bytes", record.get("size_bytes"))
        if path.stat().st_size != size:
            raise ValueError(f"byte size mismatch for {path}")
        if sha256(path) != record["sha256"]:
            raise ValueError(f"SHA-256 mismatch for {path}")
        ordered.append(path)
    if len(set(ordered)) != count:
        raise ValueError("frame aliases are not independent inputs")
    return ordered


def _read_rgb16(path, bounds=None):
    """Decode exactly one RGB16 TIFF page after bounding dimensions and dtype."""
    with tifffile.TiffFile(path) as tiff:
        if len(tiff.pages) != 1:
            raise ValueError(f"expected one TIFF page: {path}")
        page = tiff.pages[0]
        if (
            page.photometric != tifffile.PHOTOMETRIC.RGB
            or page.samplesperpixel != 3
            or page.dtype is None
            or page.dtype.kind != "u"
            or page.bitspersample != 16
        ):
            raise ValueError(f"expected actual unsigned RGB16 TIFF: {path}")
        height, width = page.imagelength, page.imagewidth
        if (
            not height
            or not width
            or (bounds and (height > bounds[0] or width > bounds[1]))
        ):
            raise ValueError(
                f"output shape exceeds reference-frame bounds: {(height, width, 3)}"
            )
        decoded = page.asarray()
        if page.planarconfig == tifffile.PLANARCONFIG.SEPARATE:
            decoded = np.moveaxis(decoded, 0, -1)
    if decoded.shape != (height, width, 3) or not np.isfinite(decoded).all():
        raise ValueError(f"invalid decoded samples: {path}")
    return decoded


def verify_output(path, bounds):
    """Require a readable, nonempty export with native 16-bit sample values."""
    image = _read_rgb16(path, bounds)
    # Direct promotion by either common mapping cannot be adequate precision.
    if not np.any(image % 257) or not np.any(image % 256):
        raise ValueError(f"TIFF contains only promoted 8-bit samples: {path}")
    return {
        "shape": list(image.shape),
        "dtype": str(image.dtype),
        "sha256": sha256(path),
        "pixel_sha256": hashlib.sha256(
            np.ascontiguousarray(image, dtype="<u2")
        ).hexdigest(),
        "bytes": path.stat().st_size,
        "minimum": int(image.min()),
        "maximum": int(image.max()),
    }


def pixel_metrics(reference, candidate, bounds=None):
    """Record unsigned 16-bit pixel errors only on like-sized geometry."""
    a, b = (_read_rgb16(path, bounds) for path in (reference, candidate))
    if a.shape != b.shape:
        return {
            "status": "different_geometry",
            "reference_shape": list(a.shape),
            "candidate_shape": list(b.shape),
        }
    summation, maximum, changed, squares = 0, 0, 0, 0.0
    # Limit extra difference buffers even for a 24 MP native frame.
    for start in range(0, a.shape[0], 64):
        band = np.abs(a[start : start + 64].astype(np.int32) - b[start : start + 64])
        summation += int(band.sum(dtype=np.uint64))
        changed += int(np.count_nonzero(band))
        maximum = max(maximum, int(band.max()))
        squares += float(np.square(band, dtype=np.float64).sum())
    return {
        "status": "decoded_pair",
        "mean_absolute_u16": summation / a.size,
        "maximum_absolute_u16": maximum,
        "rms_u16": math.sqrt(squares / a.size),
        "changed_sample_fraction": changed / a.size,
    }


def distribution(samples):
    """Retain the standard median and empirical nearest-rank P95."""
    values = sorted(samples)
    return {
        "count": len(values),
        "min": values[0],
        "p50": statistics.median(values),
        "p95": values[math.ceil(len(values) * 0.95) - 1],
        "max": values[-1],
    }


def _ratio(reference, candidate, maximum):
    if candidate is None or reference is None or reference <= 0:
        return {
            "status": "pending",
            "reference": reference,
            "candidate": candidate,
            "limit": maximum,
        }
    ratio = candidate / reference
    return {
        "status": "pass" if ratio <= maximum else "fail",
        "ratio": ratio,
        "limit": maximum,
        "candidate": candidate,
        "reference": reference,
    }


def profile_result(campaign, device, samples):
    """Account for a complete numeric profile without certifying release quality."""
    expected = campaign.schedule()
    received = [
        (sample["kind"], sample["number"], sample["target"]) for sample in samples
    ]
    invalid = any(not sample["valid"] for sample in samples)
    if (
        len(set(received)) != len(received)
        or received != expected[: len(received)]
        or invalid
    ):
        return {"status": "fail", "reason": "invalid job or inconsistent sample order"}
    if received != expected:
        return {
            "status": "pending",
            "reason": "incomplete schedule",
            "jobs": len(samples),
        }
    targets = campaign.raw["targets"]
    times = {
        target: {
            kind: distribution(
                item["elapsed_ns"]
                for item in samples
                if item["kind"] == kind and item["target"] == target
            )
            for kind in ("warm", "cold")
        }
        for target in ("reference", "candidate")
    }
    gates = {}
    for kind in ("warm", "cold"):
        gates[f"{kind}_median"] = _ratio(
            times["reference"][kind]["p50"],
            times["candidate"][kind]["p50"],
            targets[f"{kind}_completed_job"][
                "maximum_candidate_to_reference_median_ratio"
            ],
        )
    memory = (
        ("peak_rss_bytes", "peak_gpu_bytes") if device == "gpu" else ("peak_rss_bytes",)
    )
    for field in memory:
        peak = {}
        for target in ("reference", "candidate"):
            values = [sample[field] for sample in samples if sample["target"] == target]
            peak[target] = None if None in values else max(values)
        gates[field.removesuffix("_bytes")] = _ratio(
            peak["reference"],
            peak["candidate"],
            targets["peak_memory"]["maximum_candidate_to_reference_ratio"],
        )
    # A completed timing run is not evidence for representative real-image quality.
    gates["quality"] = {
        "status": "pending",
        "reason": "independent representative image review",
    }
    return {
        "status": "fail"
        if any(gate["status"] == "fail" for gate in gates.values())
        else "pending",
        "gates": gates,
        "elapsed_ns": times,
    }


def _latency_ns(entries, endpoint):
    times = [event[endpoint] - event["dispatched_ns"] for event in entries]
    if any(value < 0 for value in times):
        raise ValueError(f"{endpoint} precedes native input dispatch")
    return times


def evaluate_latency(campaign, viewer, lifecycle):
    """Reduce verified native timestamp samples; observe events in the native driver."""
    limits = campaign.raw["targets"]
    viewer_target = limits["viewer_event_to_present"]
    lifecycle_target = limits["pause_and_cancel"]
    results = {"viewer": {}, "lifecycle": {}}
    required_actions = {"source", "result", "pan", "zoom", "comparison"}
    for condition in ("idle", "loaded"):
        matching = [event for event in viewer if event["condition"] == condition]
        if len(matching) < viewer_target[
            "minimum_events"
        ] or not required_actions.issubset(event["action"] for event in matching):
            results["viewer"][condition] = {"status": "pending", "count": len(matching)}
            continue
        stats = distribution(_latency_ns(matching, "presented_ns"))
        status = (
            "pass"
            if stats["p95"] <= viewer_target["p95_max_ms"] * 1_000_000
            else "fail"
        )
        results["viewer"][condition] = {"status": status, "input_to_present_ns": stats}
    for operation in ("pause", "cancel"):
        matching = [event for event in lifecycle if event["operation"] == operation]
        if len(matching) < lifecycle_target["minimum_requests_per_operation"]:
            results["lifecycle"][operation] = {
                "status": "pending",
                "count": len(matching),
            }
            continue
        acknowledgements = _latency_ns(matching, "acknowledged_ns")
        settled = _latency_ns(matching, "settled_ns")
        if any(b < a for a, b in zip(acknowledgements, settled)):
            raise ValueError("settled lifecycle state precedes acknowledgement")
        stats = distribution(acknowledgements)
        status = (
            "pass"
            if stats["max"] <= lifecycle_target["acknowledgement_max_ms"] * 1_000_000
            else "fail"
        )
        results["lifecycle"][operation] = {
            "status": status,
            "acknowledgement_ns": stats,
            "settled_ns": distribution(settled),
        }
    return results


def merge_latency_samples(receipts):
    """Combine actual native captures only when they share an exact app build."""
    viewed, lifecycle = [], []
    identity = None
    event_times = set()
    workloads = set()
    for receipt in receipts:
        if (
            receipt.get("status") != "captured"
            or receipt.get("inputs_unchanged") is not True
        ):
            raise ValueError("incomplete native capture or input provenance")
        build = receipt["candidate"]
        current = (
            build["commit"],
            build["binary_sha256"],
            build["descriptor_sha256"],
            build["lock_sha256"],
            receipt["performance_targets_sha256"],
        )
        if identity is not None and current != identity:
            raise ValueError("native captures differ in build provenance")
        identity = current
        workload = receipt["workload"]
        if receipt["viewer"] and workload != "T50":
            raise ValueError("viewer captures require the frozen verified T50 inputs")
        for field, entries in (("viewer", viewed), ("lifecycle", lifecycle)):
            for sample in receipt[field]:
                dispatch = sample["dispatched_ns"]
                if sample.get("valid") is not True or dispatch in event_times:
                    raise ValueError("invalid or duplicate native timestamp sample")
                event_times.add(dispatch)
                entries.append(sample)
        if receipt["lifecycle"]:
            workloads.add(workload)
    return viewed, lifecycle, sorted(workloads)
