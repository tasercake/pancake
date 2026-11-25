# Native file panel and source reveal mutation receipts

Two previously missed GUI mutations were caught on
`ad81502d1e9b92c3a0d9a188691e76f12a673db3`, a test-only change based on
`8694b55`. Each one-mutant run selected a byte-identical diff from native
shard 255/256 on `287828a`. Both unmodified baselines passed all 144 CPU
tests across 19 executables, including all 18 GUI tests. Each mutant built
and failed its intended test with exit 101. Both cargo-mutants processes
exited 0 with a `CaughtMutant` record. [Machine-readable receipt](receipt.json).

| Exact archived mutation | Authentic observation |
| --- | --- |
| `desktop_menu::reveal` returns `Ok(())` | A missing source inside an owned temporary directory must return the actual path diagnostic before file-manager dispatch. The mutant reports success. |
| `panel::linux::build` returns three default GTK widgets | A real file-panel request with no display returns the `gtk_init_check` setup error and leaves its caller's completion untouched. The mutant constructs a `Dialog` without initializing GTK and fails in the child. |

The GTK test re-executes its own GUI test binary and selects exactly one
test. It clears the child's environment, preserves a supplied dynamic-linker
path, and sets an owned home and runtime directory. Its X11 backend has
no display or inherited accessibility, session, or Wayland endpoint. The
child executes the regular panel entry point. The parent checks its exit
and verifies that the expected single test ran. GTK's state stays local to
that process. The reveal assertion uses canonicalization before any
platform-specific dispatch, so it requires no file-manager service.
