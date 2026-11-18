# P8 X11 chooser handoff diagnostics

The passive C14 wrapper launches read-only XInput and X11 property observers
on the case-owned display, then execs the supplied Pancake binary under the
same PID. The first Open press and the second Open press both reached the
server at x=384, y=170 over X11 frame window 2097466. Pancake's managed Slint
XID 4194307 regained both the window-manager active state and X11 keyboard
focus 0.40 seconds before the second click. The second press and release
occurred, with no successor GTK dialog. These observations place the defect
after the X server accepted the pointer input and restored focus. They do
not determine whether winit received the individual events.
[Native monotonic trace](c14-x11.jsonl.gz)

C15 tested safe GTK transient ownership via the actual Slint X11 raw window
handle together with explicit GTK modal release before hide. The integrated
build completed. The unchanged case still timed out when reopening the
chooser, so this candidate supplies no passing evidence.
[Build](c15-build.txt)
