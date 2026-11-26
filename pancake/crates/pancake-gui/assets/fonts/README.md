# Frame Desk fonts

- IBM Plex Mono Regular and SemiBold are unchanged TrueType files from the
  [pinned Google Fonts distribution](https://github.com/google/fonts/tree/6ebe737e44fd64ee736bbc6feed6136a6dbfd222/ofl/ibmplexmono).
  [IBM Plex license](IBMPlexMono-OFL.txt)
- Space Grotesk Regular is the unchanged static TrueType file from the
  [upstream 2.0.0 release](https://github.com/floriankarsten/space-grotesk/releases/tag/2.0.0).
  Space Grotesk SemiBold is the same release's variable TrueType face at
  weight 600, instanced with FontTools 4.59.1. Add the SemiBold style to the
  weight axis in the STAT table before calling `instantiateVariableFont` with
  `updateFontNames=True` so the static face carries the correct style metadata.
  [Space Grotesk license](SpaceGrotesk-OFL.txt)

BokehForge Review bundles byte-identical copies with the same licenses for
offline use in its local server.
