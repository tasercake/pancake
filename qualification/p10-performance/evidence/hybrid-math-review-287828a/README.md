# Hybrid coefficient and expansion review at 287828a

The frozen specification leaves the pyramid expansion stencil open. Pancake’s bilinear expansion is used consistently in both directions. Its individual-source decomposition telescopes. Hybrid’s different mix factors between bands expose dyadic phase leakage in the interior, even with two perfectly aligned sources and a strictly consistent focus winner. The controlled leakage is measurable but too small in this witness to attribute the real D150 detail deficit to expansion. This audit found no violation of the frozen shape, precision, tie or detail rules from the expansion choice.

## Recorded behavior

- Pancake forms coefficients by subtracting bilinearly expanded coarse pixels from a binomially reduced image pyramid. It uses that same expansion on reconstruction. For each band the score is a Gaussian reduction of the locally pooled, full-resolution modified Laplacian. Normalization blends the strongest source coefficient with the sum-weighted coefficients. The weighted mean has fraction 0.14 at detail bands 0 and 1 and 0.60 at all lower bands and the base. Exact zero weight selects the first input. [Pyramid](../../../../pancake/src/fusion/advanced/pyramid.rs#L111) · [Coefficient selection and normalization](../../../../pancake/src/fusion/advanced/levels.rs#L190) · [Band traversal](../../../../pancake/src/fusion/advanced/mod.rs#L269)

## Closed-form scale and phase witness

Take two normalized RGB sources constant across rows, with brightness `0.5 + a·f(x)` and `0.5 + b·f(x)`. Set `f(x)=cos(πx/4)`, `a=0.32`, `b=0.10`. The pooled absolute second differences are positive and have ratio `a:b` everywhere in the interior. Linear weight reduction preserves this ratio. The sharp source wins all bands, alignment is exact, and both sources have zero third harmonic.

The contrast of the weighted mean is `m=(a²+b²)/(a+b)=0.26761905`. The two finest bands have contrast `βf=0.86a+0.14m=0.31266667`. Lower bands have `βc=0.40a+0.60m=0.28857143`. Let `D` be one binomial reduction and `U` one expansion. Telescoping within each band group gives the final contrast signal `βf·f + (βc−βf)·U²D²f`. The quarter-grid signal is alternating, with magnitude `cos⁴(π/8)cos⁴(π/4)=0.18213835`. The next reduction removes it. With bilinear expansion the alternation becomes a triangle. Its signed third cosine component has relative gain `(1−1/√2)/2` before multiplying by the quarter-grid amplitude. With the pinned expansion stencil the relative gain is `cos⁴(3π/8)/2`.

| Expansion with the same reduction and coefficients | Fundamental | Third harmonic | Third harmonic in RGB16 codes |
| --- | ---: | ---: | ---: |
| Pancake bilinear interior | 0.30892071 | −0.0006427054 | −42.1197 |
| Pinned Gaussian interior stencil | 0.31106798 | −0.0000470610 | −3.0841 |
| Joint two-pixel input shift, either stencil | 0.31266667 | 0 within roundoff | 0 within roundoff |

The Gaussian column substitutes only the pinned stencil into the same controlled math. It makes no prediction about the full pinned Hybrid renderer with its different band weights. The two-pixel joint shift moves the cosine to a sine on the original grid and puts a null at the second decimator. The output follows the known grid phase without any change in source alignment. Both signals stay within the export range. [Independent periodic convolution](periodic_expansion.py.txt) · [Scalar result](periodic-expansion.json)

A follow-up can exercise the real locked CLI with two 257×17 RGB f32 NPY inputs, `--method hybrid --cpu --kernel-size 6 --pyramid-levels 3 --alignment-mode off --slab-size 0 --bit-depth 16`, and independently decode the RGB16 TIFF. Project x=96..159 on the central rows onto the fundamental and third cosine, then repeat on the transposed pair, the joint two-pixel shift, and a two-identical-source control. The center avoids the finite-image border stencil. One RGB16 code bounds the first-harmonic or third-harmonic error contributed by rounding alone. CPU and observed physical GPU can use the same sources and projections. This is a targeted response test for any future blend or expansion change. The fixture and resulting product observations are still pending.

## Attribution on real data

The earlier D150 geometry diagnostic removed a false 129.8-pixel late transform jump and expanded the crop. Its four common regions still recovered 0.685–0.704 of the pinned local detail, near the sealed 0.686–0.706. A separate fixed-input diagnostic isolated fusion with 40 identical, independently registered Lanczos4 RGB16 sources and no internal alignment or crop. Historical stock signed projections in two raw-selected regions were 0.308888 and 0.237393 versus 0.392158 and 0.298540 in the pinned render. A band-local variance experiment raised unsigned response slightly and lost signed phase in both regions. The same source differences can therefore include phase and contrast across broader bands. The retained evidence does not apportion the real deficit between band selection, mixing, reconstruction and the reference fine enhancement. [D150 geometry](../../../../pancake/docs/d150-geometry-audit/prototype-ff2c037/README.md) · [Identical-input D150](../d150-band-focus/README.md)

Reproduce the new scalar result from this directory with `python3 periodic_expansion.py.txt | diff -u periodic-expansion.json -`. No frozen thresholds, source or packages changed.
