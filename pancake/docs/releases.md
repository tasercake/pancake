# Build and publish Pancake packages

In GitHub, open [Actions → Pancake packages](https://github.com/tasercake/pancake/actions/workflows/pancake-release.yml) and choose **Run workflow**. The workflow starts only when manually dispatched. By default it builds all three packages and uploads them to the Actions run for review.

| Platform | Package | How to run |
| --- | --- | --- |
| macOS on Apple Silicon | ZIP containing `Pancake.app` and `pancake-cli` | Extract the ZIP and move the app to Applications. The app is ad-hoc signed and has no Apple notarization. |
| Ubuntu 24.04 x86-64 | `pancake_<version>_amd64.deb` | Install with `sudo apt install ./pancake_<version>_amd64.deb`. Launch Pancake from the application menu or run `pancake-cli` in a terminal. |
| Windows x64 | Portable ZIP with `pancake-gui.exe` and `pancake-cli.exe` | Extract both executables before launching. The executables are unsigned and use the x64 Microsoft Visual C++ runtime. |

Each builder uses the committed Rust toolchain and Cargo lockfile. The macOS release build also links a pinned static copy of Zstandard. Before it uploads its package, it runs a two-frame CPU stack with the packaged CLI. The Linux and macOS jobs retain package receipts on the Actions run. Desktop and accelerator verification is covered separately in the [developer guide](../DEVELOPING.md).

Successful runs on the default branch save compiled dependencies for subsequent runs with matching build inputs and runner images. Run the review build on the default branch to warm the cache for publishing. Each run rebuilds the Pancake binaries and verifies the packages it produces.

When packaging repeatedly on an Apple Silicon Mac, pass `--target-dir "$PWD/target"` to `scripts/package_macos.py` from the `pancake/` directory to reuse Cargo build output. Use a target directory for that checkout and its native linker configuration, and give each package a fresh output path.

## Publish on GitHub

1. Set the same three-component version in the core, CLI, and GUI Cargo manifests. Update the CLI and GUI's exact core dependency versions and commit the matching `Cargo.lock`.
2. Run the package build on the commit you want to release and review the artifacts. Run [Pancake assurance](https://github.com/tasercake/pancake/actions/workflows/pancake-assurance.yml) and the appropriate native checks when qualifying a release.
3. From the default branch, dispatch **Pancake packages** with **Publish the verified packages to GitHub Releases** checked. The prerelease checkbox is on by default.
4. After all three builds and smoke tests succeed, the workflow creates the `v<version>` tag at the dispatched commit and publishes the three packages with `SHA256SUMS.txt` and a link to the corresponding source. Each publish uses a fresh version tag.

From the GitHub CLI, run the build-only or publishing form.

```sh
gh workflow run pancake-release.yml --ref main
gh workflow run pancake-release.yml --ref main -f publish=true -f prerelease=true
```
