# Pancake P8 desktop services

The test-first history checkpoint `cb2dc7f` failed for the missing
`RecentFolders` API. The four tests pass on the pinned implementation and
exercise canonical Unicode MRU history, failure-preserving writes, malformed
and oversized files, and bounded entries. Scoped workspace Clippy passed.
[History red](history-red.txt) · [History tests](history.txt) · [Clippy](clippy.txt)

An attached GTK submenu reported `AXMenu` for its owner despite appearing at
the pointer. The revised File menu exposes the real actionable
`AXMenuItem` and expands a second native File popup with recent entries.
[Initial role result](submenu-role-red.txt)

The private native probe cancelled and reopened a chooser in the same
process, accepted an actual original Unicode PNG, selected its Unicode parent
directory and resolved a Unicode save path. The save probe wrote a marker
through the chosen path. Image encoding is verified separately by the stacking
suite. The probe selected a native Recent item, cleared the history on disk,
read the disabled empty item, revealed the actual source in Thunar and
observed native focus. File > Close Window exited normally. The source PNG
was byte-identical after the actions. The native trace contains real
AT-SPI snapshots and physical input.
[Panel results](panels.json) · [Menu and reveal results](menus.json) ·
[Native trace](ui-actions.jsonl.gz) · [Shutdown](shutdown.json) ·
[Binary and source hashes](probe-hashes.txt)

## Reproduce the scoped probe

From a checkout of the source with the evidence files available:

The native product recent-folder and source-context cases still need the
Slint controller. macOS and Windows execution was unverified. The observed
Linux host used X11. The isolated service probe supplies no product case
passes or package credit.
