# Independent source checks

Accessed 2026-11-15. These checks support final synthesis and cross-check agent findings; they are not additional independent experiments.

## Official macro software documentation

- [Zerene Stacker FAQ](https://zerenesystems.com/cms/stacker/docs/faqlist): vendor documents a tradeoff between PMax detail retention and increased noise, contrast, color changes, and inversion halos. DMap better preserves smoothness and color but can lose detail. Source type: original vendor manual; evidence inspected: retrieved search excerpts. These are documented behavior claims, not a controlled comparison conducted here.
- [Zerene retouching tutorial transcript](https://zerenesystems.com/cms/stacker/docs/videotutorials/retouching002/transcript): vendor's worked example describes broad dark halos and increased pixel noise in a PMax output. Source type: original tutorial; evidence inspected: retrieved search excerpts.
- [Helicon Focus parameters](https://www.heliconsoft.com/helicon-focus-main-parameters/): vendor describes a fine-detail versus halo/noise tradeoff as focus-measure radius changes, and increased contrast/glare for its pyramid method. Source type: original vendor manual; evidence inspected: retrieved search excerpts.
- [Helicon Focus 9 guide](https://www.heliconsoft.com/focus/help9/english/HeliconFocus.html): current guide is available separately from the version 8 guide. Search excerpts mention exporting effective contributing source regions. Full capability and semantics require the algorithm/measurement reviews' closer inspection.

Inference: an evaluation based only on edge strength or apparent crispness can reward a documented failure mode. The size and practical ranking effect must still be tested; documentation alone does not calibrate any metric.

## Clean source pixels versus recoverable information

[Zerene DMap tutorial](https://zerenesystems.com/cms/stacker/docs/tutorials/tutorial003), section “Dealing With DMap's Limitations,” inspected as full retrieved page text. Its foreground-petal/background-leaf example shows that a focused background can be contaminated by foreground defocus in every candidate source. Copying one sharp pixel cannot produce the desired value. The tutorial also describes combining color and luminance information across frames with PMax. Therefore, absence of a clean source pixel does **not** establish impossibility for every reconstruction method. A benchmark must distinguish selection-only support, model-based reconstructability, and information genuinely absent from all measurements.
