# T50 typed TIFF and resident RGB scan evidence

The exact combined candidate `e5b62606cbdd9b41758ccee6efafec8b02fe4ad1`
removes two redundant full-resolution traversals. The typed tiled TIFF
completion already holds a checked, correctly sized buffer written from
finite U16 values. The Hybrid resident arm consumes the same immutable
registration upload and checks packed shape, stride, type and originating
compute service before submission. General float construction and public
Hybrid host uploads retain finite-value and geometry validation.

The TIFF branch `c6d4879` moved the actual typed TIFF constructor count
from one to zero. Its native oracle compared 3,158,025 sample bits, ICC
and U16 provenance, and the diagnostic for a truncated final tile. Both
revisions reached public float validation six times in the separate
control. The resident branch `0b4aeb3` failed the private device-only
input witness on the old source and produced 741 bit-exact CPU values
on the candidate. Its permanent test rejects short and NaN host samples,
then accepts a valid frame. [Resident witness](../../p10-resident-validation/README.md).

## Physical T50 Hybrid diagnostic

The exact `fb9c399` Debian CLI and candidate ran balanced stock, candidate,
candidate, stock in one exclusive window on the same RTX 3090 with
NVIDIA driver 580.178.04. Both used the frozen T50 Hybrid settings,
CPUs 0–31, 32 threads and per-process fresh caches. The frozen measurement
helper retained launch-to-`wait4` time, CPU and GPU boundary observations
and raw resource streams. The four launches completed in an 84.683 s
measurement window. Starting GPU temperatures were 45, 52, 52 and 49 °C.

| Pair | Stock | Candidate | Reduction |
| --- | ---: | ---: | ---: |
| Stock first | 21.966022 s | 19.544322 s | 2.421700 s (11.0%) |
| Candidate first | 22.045896 s | 19.513047 s | 2.532849 s (11.5%) |

All four encoded RGB16 and decoded sample digests matched each other
and the earlier exact T50 Hybrid result. Every completion recorded
50 exact reference-to-source maps and 50 exact sequential maps. The
whole output covered 3998 × 5998 RGB16 pixels. The generated truth
comparison used the one-pixel symmetric crop and scored all 50 original
focus regions. The whole centered overlap MAE was 843.328 u16 codes.
All 52 input TIFFs were rehashed, and the four produced TIFFs were
rehash-verified before removal. The input stage stayed with its owner.

The archives retain the exact source and binary identities, measured
resource traces, selected native case reports and raw completion events.
They exclude generated input, exported TIFF data, binaries and ambient
environment capture. The locally retained CLI is
`artifacts/pancake-performance/binaries/t50-dual-scan-e5b6260/pancake-cli`.
Run `python3 audit.py.txt` from this receipt to recheck the archived
case roster, backend outcomes, test totals and all four resource streams.
