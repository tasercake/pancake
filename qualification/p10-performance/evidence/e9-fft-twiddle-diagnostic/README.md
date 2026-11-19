# E9 FFT twiddle experiment

Reusing each stage's sine and cosine values across independent butterflies
preserved the real E9 export and yielded no warm CUDA latency gain. The offset-
first branch remains experimental. [Phase and compiler attribution](../e9-cuda-phase-attribution/README.md)
records the first-use cost that motivated the test.

| Native CUDA build | Warm median |
| --- | ---: |
| Historical application control `12f2151` | 452.966 ms |
| FFT candidate `9a8c1a9` | 457.087 ms |

The median ratio was 1.00910 and the median candidate-minus-control difference
within pairs was +2.806 ms. Each binary completed two warmups and five measured
fresh-process launches in alternating order. The blocking Linux `wait4` timer
covered process creation through full process exit and used a separate deadline
for the run. The RTX 3090 at PCI `0000:01:00.0` had driver `580.178.04` and
recorded 69°C and P2 before and after. Both
process censuses were empty. All 14 jobs reported actual physical CUDA with
zero fallbacks and nine exact identity transforms. Each genuine 337 × 510
RGB16 TIFF matched the prior E9 file and decoded-pixel hashes.

The earlier run with the same 14 successful exports used Python's timed wait.
Its polling capped at 50 ms, so those recorded timings were excluded from the
comparison. The archive retains both raw runs. The nine ordered Balloons
photographs were hashed before and after every group. The download and cleanup
also verified the original source archive. Cleanup removed 71,500,200 bytes
across the source archive, notice, and photographs.
