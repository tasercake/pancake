# P9 Slint interpreter boundary

Keep the safe Slint 1.19 interpreter for Pancake's embedded workspace, with a
built-in source URI and empty filesystem include and library paths. The desktop
application uses the presenter and shared job engine. The decision accepts
Slint's runtime compiler and patched Rowan 0.17.0 as dependencies for this
specific, repository-owned Slint source. Re-review the boundary if Pancake adds
external Slint source, imports, project files or live preview.

The initial desktop passed an empty source path to the interpreter. The pinned
Slint type loader searched the working directory before its built-in
`std-widgets.slint`. A synthetic local widget file exploited this resolution
order in the old binary. The application read the file and failed compilation
because the widget exports were absent. Changing the source name to
`builtin:/pancake/workspace.slint` selects the compiler's virtual path scheme.
Slint resolves the workspace's one import from the bundled Fluent standard
widgets.

| Debug source | Working directory | Native window | Widget file read |
| --- | --- | --- | --- |
| `8f1da27` | Empty | Opened | No |
| `8f1da27` | Local `std-widgets.slint` | Exit 1 before window | Yes |
| `fa885f1` | Empty | Opened | No |
| `fa885f1` | Same local `std-widgets.slint` | Opened | No |

Each observation used a fresh private Xvfb and D-Bus process. `strace` watched
file opens and `xwininfo` observed the real desktop window. The fixtures,
commands, executable hashes, lock hashes and compact traces are retained.
[Interpreter boundary receipt](p9-slint-boundary-evidence/receipt.json).

The inspected GUI has one interpreter compiler call and one declared Slint
import. The GUI parses command-line paths as image sources after building
and creating the workspace. Normal job state, image content and user
preferences reach real component properties and callbacks. The application
does not use `build_from_path`, a custom file loader or another source import
path. The dependency graph puts Rowan in the GUI through the Slint interpreter
and its compiler. Slint and Rowan are absent from the Linux headless CLI graph.
Slint, CubeCL and the application resolve one wgpu 30.0.2 dependency. [Rowan
graph](p9-slint-boundary-evidence/rowan.log).
[wgpu graph](p9-slint-boundary-evidence/wgpu.log).

The P0 upstream Rowan probe still has a strict-Miri ARC retag finding under
the Stacked Borrows model. The representation patch leaves the relevant ARC
implementation unchanged. Restricted source origins reduce the parser's input
surface. The release still carries that runtime dependency and its known
limitation. Keep the first-party unsafe prohibition and integrated workspace
assurance gates. This review establishes source origins on the two tested
debug binaries. Verify the packaged release binary and confirm its graph
matches the reviewed dependency resolution. [Rowan qualification
evidence](p0-kernel-qualification.md#rowan-layout-boundary).
