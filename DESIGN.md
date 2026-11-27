# Pancake Stacker and BokehForge design specification

Frame Desk is the shared design for Pancake Stacker and BokehForge. The apps
look like tools for working with photographs. A neutral graphite workspace
keeps the image central, compact controls make repeated work easy, and the
original Pancake Stacker artwork gives welcome, About, and release surfaces their
identity.

## Workspace

- Use contiguous panes with shared edges. Mount a compact command bar at the
  top, the preview and inspector side by side, and a source filmstrip beneath
  them. Keep the preview flexible and the inspector wide enough for its
  settings. Let the filmstrip span the workspace.
- Group toolbar commands by task. Separate groups by a gap or one fine
  divider. Keep frequent commands near the view they affect. Use short labels
  for unfamiliar symbols and expose names and shortcuts to assistive
  technology.
- In Pancake Stacker, keep stack method, alignment, and output controls in the
  inspector. Put the stack action at the bottom of the inspector. Keep
  retained results and the complete source list accessible. Open Preferences,
  About, and export options using the same surfaces and control styles.
- In BokehForge, keep the source frame, stacker output, and available
  ground-truth comparison aligned on the same matte. Put evaluation and rating
  controls in the inspector. Keep scene navigation, evaluation progress,
  result details, and review overlays accessible. At narrow browser widths,
  stack the panes in reading order and let the filmstrip scroll horizontally.
- Use real data and real application states. Give empty and loading
  workspaces a clear next step. Preserve selection, focus, shortcuts,
  cancellation, and progress through layout changes. Deliver state updates
  through the existing event channels.

## Palette

Graphite is the default. All base surfaces are achromatic and opaque. Use
small changes in lightness to distinguish adjacent surfaces and reserve the
accent for actions and states.

| Role | Dark value |
| --- | --- |
| Frame and title surface | `#1e1e1e` |
| Command bar | `#232323` |
| Image matte | `#191919` |
| Inspector and pane headers | `#2b2b2b` |
| Filmstrip | `#252525` |
| Fields and inactive controls | `#343434` |
| Pane and group dividers | `#565656` |
| Field boundaries | `#7f7f7f` |
| Primary text | `#f2f2f2` |
| Secondary text | `#c2c2c2` |
| Neutral icons | `#dedede` |
| Photo keyline | `#101010` |
| Default accent | `#ffffff` |

- Provide one user-configurable accent in each graphical workspace. Store the
  preference locally and use white when there is no valid preference. Derive
  the applied color from that one input. In dark mode, lighten an accent
  toward white as needed to reach 4.5:1 against the lightest control surface.
  Choose black or white text on the applied fill by contrast.
- Keep normal text and control labels at 4.5:1 or higher against their
  surfaces. Keep meaningful icons, field boundaries, selection marks, and
  focus indicators at 3:1 or higher against adjacent surfaces. Use shape,
  position, and meaningful text alongside fill. A dark keyline separates a
  light selection outline from a bright thumbnail.
- Use a filled accent for the primary action and selected numeric control.
  Use the same accent in a thin underline, selected frame outline and number,
  checkmark state, or progress fill. Keep peer controls neutral. Report
  progress, success, and errors with words and symbols.
- Preserve native theme choices where already supported. Keep the secondary
  light theme neutral and apply the same contrast rules against its actual
  surfaces. System-owned menus, window controls, file panels, and terminal
  colors follow the host.

## Typography

- **Orient and inspect.** Use IBM Plex Mono at weight 400 for labels,
  filenames, values, commands of equal priority, and status readouts. Use
  weight 600 for pane headings and selected identifiers. Use tabular
  numerals for comparisons and frame counts.
- **Decide and commit.** Use Space Grotesk semibold for the short instruction
  that advances the current task. In the workspace that is **Start stack** in
  Pancake Stacker and **Save rating** in BokehForge when a rating can be saved. In a
  dialog the confirmation action takes that role. A short heading that asks
  the user to resolve a blocking decision can use the same emphasis.
- Assign the role by the task. **Export** in the workspace toolbar is a peer
  command. **Export** in the export confirmation is the decision. Keep the
  emphasis face close to the workspace face in apparent size and stroke
  weight. Give each active pane or dialog one clear primary action.
- Use regular Space Grotesk for longer help and explanatory copy. Keep
  identifiers in IBM Plex Mono. The pixel lettering belongs to the original
  wordmark. Use host fonts for native system chrome and terminal output, and
  allow fallback glyphs for filenames and content outside the bundled fonts.
- Bundle the fonts with graphical apps for offline use. Keep their SIL Open
  Font License notices with their distributions.

## Components

- Use a compact desktop rhythm with regular alignment. Start with a 4 px
  spacing unit, 8–12 px between grouped controls, 12–16 px pane padding,
  13–14 px workspace text, and approximately 28–36 px control heights at
  normal desktop scale. Retain unclipped text and usable targets when scaled.
- Use shallow radii of about 4 px on fields, buttons, and the current
  thumbnail. Use square image edges and fine pane dividers. Group related
  controls with spacing and alignment. Avoid applying a heavy border to each
  section.
- Draw small consistent line icons for open, source frames, stack,
  compare, crop, zoom, and export. A stack of offset outlines is the
  workspace motif. Use existing platform icons when a control is host owned.
- Give focus a visible outline distinct from a selection fill. Give the
  current frame a clear border and its index. Keep thumbnails in capture
  order and show the frame count beside the strip. Support keyboard movement
  and filenames in accessible labels. Leave a stable labeled placeholder
  while a thumbnail is unavailable.
- Keep photo previews and thumbnails on a neutral matte. Preserve
  aspect ratio and independent photo color. Keep comparison lines thin and
  legible over both light and dark detail. Use separate, short captions for
  paired review images and keep any diagnostic overlays explicitly selectable.
- Keep dialogs close to the work. Retain focus on return, group field labels
  and values, and apply the primary action style to the confirmation.
  Use neutral borders and readable feedback on all floating surfaces.

## Artwork and host interfaces

- Use the original [Pancake Stacker wordmark](assets/pancake-stacker-wordmark.png)
  at a readable size on welcome, About, or release surfaces. Leave its
  illustration and lettering intact. Use the supplied app icon where a
  platform expects one. Keep everyday working chrome plain.
- Use the BokehForge name and the same typography, palette, and components
  for review. Its workflow determines the labels and panes.
- In terminals, use the terminal's type and colors. Apply the hierarchy
  through concise sections, readable labels, and aligned values. Preserve
  machine-readable output, error streams, and redirection semantics.
- In Lightroom, use Lightroom's native controls and dialogs. Use the product
  name and direct action labels consistent with Pancake.
