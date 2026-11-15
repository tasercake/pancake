# Independent reading: real-burst focus stacking

Source: Araujo, Ponce and Mairal, [Towards Real-World Focus Stacking with Deep Learning, v1](https://arxiv.org/html/2311.17846v1). Accessed 2026-11-15; relevant original HTML sections inspected during phase 2.

- Table 2 reports 84 training bursts and 10 test bursts, divided between two lenses. This is a burst count, not an independent crop count.
- Section 3 says the macro-lens sequences can lack full focal coverage even with 30 frames.
- Sections 3–4 use Helicon's pyramid output as a pseudo-reference. Higher agreement measures teacher imitation; it does not independently prove superior fidelity.
- Section 5 explicitly notes registration limits and reproduction of pseudo-reference mistakes. Source geometry can mix rays from different scene points.
- Reported pairwise comparisons using only burst endpoints cannot establish full-stack focal coverage or long-stack superiority.

Implication for later review: retain original provenance, identify missing focus coverage, and keep teacher agreement distinct from independent quality. Verify current repository release and data terms before acquisition.
