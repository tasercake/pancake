# Immediate viewer commands

The [matched redraw experiment](../p10-viewer-redraw-evidence/README.md)
measured a visible P95 around 41 ms with both binaries. The pinned Fluent
Button starts 150 ms background, border, and text animations when its
states change. Retained-result, view mode, and fit/zoom/navigation
commands all used this component. This created a narrower source-level
hypothesis about unnecessary decorated frames during viewer interaction.

At `9681d09`, a test drove the real embedded workspace through its
offscreen Slint software renderer. Scrolling first changed the displayed
photograph. Moving the pointer to the Fit command then left an active
animation in the window and failed the test.
[Exact failing test](red.log).

At `5f6ce9a`, viewer commands use an immediate-state control styled by
the application's palette. The control keeps its accessible button
identity, enabled and checked state, default action, focus target,
pointer activation, and Space and Return activation. The test passed
after actually clicking Fit, checking that the portrait returned to its
original visible size, then navigating by Tab to the 1:1 command and
activating it with Return. It checks the displayed pixels and an absence
of active decoration at both states. All 16 GUI tests, effective strict
GUI Clippy, formatting, and reviewed source inventory passed.
[Exact CPU checks](green-and-clippy.log.gz).

## Frozen physical replay

The candidate passed the 33 ms P95 gate during both idle viewing and
real GPU stack processing. Each arm captured all 400 valid events,
with 200 idle and 200 loaded events and 40 of each action per condition.
All 2,240 observed RGB patches independently matched the original
calibrated images within one level per channel. The frozen tolerance
is two levels.
[Independent verification](summary.json) and [per-action distributions](per-action.json).

| Condition | Packaged P50 | Immediate P50 | Packaged P95 | Immediate P95 | Target |
| --- | ---: | ---: | ---: | ---: | ---: |
| Idle | 27.613 ms | 19.714 ms | 36.323 ms | 20.793 ms | ≤33 ms |
| Loaded | 29.351 ms | 20.668 ms | 42.459 ms | 23.827 ms | ≤33 ms |

The count over 33 ms fell from 33 to 1 idle and from 82 to 5 loaded.
The first cold source switch remains included at 667.520 ms and
659.484 ms respectively. Both collectors exited 0. The unchanged
process reducer writes the viewer outcomes above and exits 1 because
the independent pause/cancel workload remains pending in these
viewer-only receipts.
[Packaged reduction](a.frozen-reduction.json) and [immediate reduction](b.frozen-reduction.json).

## Provenance and replay

- Candidate `6489af6ea4e9611d5ef26bbba6109dbf0fbe8947` is the
  `5f6ce9a` product source with the offscreen evidence added. It was
  built with locked release Cargo 1.100.0. Executable SHA-256
  `ad97c445535aa822677f40afc59d48c21bf72cc9e9af7d521c9f2cf571296ac4`.
  Both arms used Cargo lock SHA-256
  `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
  [Build receipt](b.build.json), [release log](release.log.gz) and
  [source and protocol hashes](source-and-protocol.txt).
- Both ran on an AMD Ryzen 9 7950X3D with NVIDIA RTX 3090 driver
  580.178.04 in separate private 1440 × 1000 Xvfb sessions. Each
  recorded a real `wgpu-vulkan` stack completion on PCI
  `0000:01:00.0`. Loaded events sampled Decode/Fuse in counts 120/80
  on the control and 142/58 on the candidate. Maximum observation gaps
  were 1.205 ms and 1.838 ms respectively. On 2026-11-18 the control
  ran 18:03:38–18:08:17 SGT and the candidate 18:08:17–18:12:34 SGT.
  [Hardware and worker receipts](summary.json).
- Both consumed the same ordered 50 T50 focus TIFFs. The collector
  verified unchanged input bytes before and after each arm.
  Materialization receipt SHA-256
  `8e4742b2480e041ca95cceeb023627a4c3c69c27f618e3bc13c64874a75ff468`.
  All original 19 PNGs and scenario steps were preserved. Only the
  binary SHA-256 and source commit were rebound per arm.
  [Original calibration hashes](../p10-viewer-redraw-evidence/calibration-preflight.json).
