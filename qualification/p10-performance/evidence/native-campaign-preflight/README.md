# Sealed Linux native campaign preparation

The following generates the two lifecycle scenarios without launching the GUI
or sampling any latency.

```sh
campaign=qualification/p10-performance
identity="$campaign/evidence/native-campaign-preflight/identity.json"
python "$campaign/native_campaign.py" lifecycle --identity "$identity" \
  --workload lifecycle --output artifacts/fixture-64.json
python "$campaign/native_campaign.py" lifecycle --identity "$identity" \
  --workload D150 --output artifacts/real-D150.json
```

Each scenario requests 30 pauses and 30 cancellations through the native
controls. It resumes the same worker after each pause, dispatches cancellation
during an observed numerical stage, requires truthful same-job acknowledgement
and actual reusable Idle, then starts another worker generation. The last real
job completes to verify that the app remains usable and to record the actual
Vulkan runtime and physical device. The 64-path fixture uses the original
Laplacian and Breathing settings. D150 uses Hybrid and Breathing on all 150
original JPEGs from the ordered frozen manifest.

The viewer scenario generator prepares 40 events each for source selection,
result selection, pan, zoom, and comparison drag per condition. Collect each
of the 200 idle events and 200 loaded events only after fixing their before and
after pixel witnesses. This needs a private physical-wgpu calibration session,
an ownership-verified rematerialization of the original 24 MP T50 TIFFs, and an
independently decoded real result from the same GUI configuration. Use those
decoded images and the actual viewport geometry to check each witness, including
the comparison division and zoom/pan coordinate changes. A previous Slint
software-renderer pixel calibration supplies a method check. The native Vulkan
viewport still needs its own fixed RGB witnesses. Xvfb records visible virtual
desktop presentation and does not observe a physical monitor’s scanout.

Ruff and all 18 focused tests passed, including exact sample counts, fixed
pixel distinction, and rejection of mismatched physical PCI provenance. The
prepared scenarios supply zero native latency samples.
