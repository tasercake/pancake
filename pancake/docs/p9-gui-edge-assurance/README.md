# GUI profile and error-recovery assurance

The test-only revision `aac05e4` adds three regressions using the actual
GUI application stores and embedded Slint bindings.

- Opening an existing file or a missing directory reports the applicable
  folder error. Both the current recent-folder list and the committed
  history bytes retain the previous valid directory.
- A valid preferences document of precisely 64 KiB retains the selected
  CPU option. One additional byte yields the size error without editing
  the document.
- The production preference callback reports malformed data at startup
  and exposes default selector states. It reports a failed atomic save
  over a conflicting directory and preserves that directory's data.
  After the obstruction is removed, the same workspace saves the revised
  preferences, clears its error message, and releases its callback and
  adapter on drop.

| Production file | Code regions | Originally covered by these unit tests | Covered with the new tests | New records with a zero count in the archived combined report |
| --- | ---: | ---: | ---: | ---: |
| `history.rs` | 112 | 87 | 90 | 3 |
| `main.rs` | 117 | 11 | 51 | 7 |
| `preferences/mod.rs` | 73 | 37 | 49 | 3 |

The reducer checks the production source at the archived revision before
matching coordinates. The matched profiles count production code regions,
including nested expressions on the same line. The scoped gains leave
other production regions uncovered.
[Region deltas and hashes](delta.json), [baseline LLVM export](baseline-coverage.json.gz),
and [expanded LLVM export](added-coverage.json.gz).

Rust and Cargo 1.100.0 on the Ryzen 9 7950X3D built the test object in two
jobs. From `pancake/`, source `cargo llvm-cov show-env --sh` and run
`cargo test --locked --no-run -p pancake-gui --bin pancake-gui --message-format=json`.
The replay helper runs both selections and exports the same ELF with
each LLVM profile. The effective-policy GUI Clippy check, formatting,
and reviewed source inventory also passed.
[Replay](compare-profiles.sh.txt), [reducer](compare-regions.py.txt),
[toolchain](toolchain.txt), [object hash](test-object.sha256),
[build log](build.log.gz), and [Clippy log](clippy.log.gz).
