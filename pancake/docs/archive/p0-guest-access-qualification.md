# P0.02 dedicated macOS guest access

## Result

| Check | Actual observation |
| --- | --- |
| Resource configuration | 4 vCPUs, 8 GiB RAM, configured display 1440×1000. These resources were reserved only during the bounded boot/check interval. Actual rendered framebuffer was not measured. |
| Session | Console user `admin`; live `launchctl print gui/501` domain. This proves an Aqua session exists, not that a Pancake window was operated. |
| Native platform | `Darwin arm64`, macOS 26.6.2, build 25G83. SIP enabled. |
| Executed native tools | uv 0.12.17, Apple Swift 6.3.3, arm64 macOS target. |
| Existing runtimes | Managed Python 3.12.14 path and suite/reference-adapter `.venv/bin/python` paths exist. Their Python versions/dependency locks were not re-executed or reconciled in this check. |
| Current permission observation | `CGPreflightScreenCaptureAccess` returned true for the SSH-launched probe. This does not qualify a new LaunchAgent/Pancake process identity. |
| Prior permission evidence | Existing guest `.bootstrap/permissions.json` records screen capture, AX trust/post-event access, automation and screenshots true. Record is older; current AX/Automation/LaunchAgent identity was not requalified. No native helper was found in the SSH process's temporary cache directory. |

## Commands and boundary

No guest clone, image download, package installation, source synchronization,
password guessing, Recovery boot, host desktop control, or host TCC change was
performed. Tart output was retained only in an ignored mode-0600 private log;
its contents are not published. The task respected the unrelated guest's state.

## Remaining qualification

Dedicated guest SSH/Aqua/native-tool access is demonstrated. Before claiming a
GUI lane pass, run the declared runtime and native helper in the actual suite
LaunchAgent identity, verify current Accessibility/Automation/screen permissions,
prepare the real Pancake binary and adapter, and execute the unchanged cases.
A macOS virtual guest does not qualify physical-GPU execution or native
NVIDIA/AMD/Intel/Windows lanes.
