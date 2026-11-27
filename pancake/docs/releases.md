# Build and publish Pancake packages

In GitHub Actions, open **Pancake packages** and choose **Run workflow**. Select **prerelease** or **stable** on the default branch to publish. The default **build-only** mode uploads the packages to the Actions run. [Pancake packages workflow](https://github.com/tasercake/pancake/actions/workflows/pancake-release.yml)

| Platform | Package | How to run |
| --- | --- | --- |
| macOS on Apple Silicon | `pancake-<version>-macos-arm64.dmg` containing `Pancake.app` and `pancake-cli` | Open the DMG and drag the app into Applications. The app is ad-hoc signed and has no Apple notarization. |
| Ubuntu 24.04 x86-64 | `pancake_<version>_amd64.deb` | Install with `sudo apt install ./pancake_<version>_amd64.deb`. Launch Pancake from the application menu or run `pancake-cli` in a terminal. |
| Windows x64 | `pancake-gui-<version>-windows-x64.exe` and `pancake-cli-<version>-windows-x64.exe` | Run either executable directly. Both are unsigned and use the x64 Microsoft Visual C++ runtime. The release includes a separate Windows notices archive. |

Each builder uses the committed Rust toolchain and Cargo lockfile. The macOS release build also links a pinned static copy of Zstandard. Before it uploads its package, it runs a two-frame CPU stack with the packaged CLI. The Linux and macOS jobs retain package receipts on the Actions run. Desktop and accelerator verification is covered separately in the [developer guide](../DEVELOPING.md).

Successful runs on the default branch save compiled dependencies for subsequent runs with matching build inputs and runner images. Each run builds and verifies its own Pancake binaries.

When packaging repeatedly on an Apple Silicon Mac, pass `--target-dir "$PWD/target"` to `scripts/package_macos.py` from the `pancake/` directory to reuse Cargo build output. Use a target directory for that checkout and its native linker configuration, and give each package a fresh output path.

## Publish on GitHub

Select **prerelease** or **stable**, then click **Run workflow**. Actions assigns the version, writes the changelog, and verifies the packages before it stages and publishes the GitHub Release. Stable builds also run the hosted release assurance checks. The tag points to the exact commit used for the builds, including the versioned Cargo manifests and lockfile.

- **Prerelease** uses `v<version>-rc.<number>` and describes changes since the newest published prerelease or stable release on the source history.
- **Stable** uses `v<version>` and describes all changes since the previous stable release, including changes in intervening prereleases. The first stable release uses the full history.
- **Build-only** checks and stages the candidate packages as Actions artifacts for review.

Release notes start with a download table for the three platforms and a checksum link. The changelog links the complete set of included changes. Copilot adds focused highlights and recommends a version bump. The workflow reserves a new tag, continues an open release-candidate series, and uses a minor bump for breaking changes during initial `0.x` development. Git and conventional commit markers provide the fallback when Copilot is unavailable. Any change that failed verification stops publication.

If publication is interrupted after the draft is staged, use **Re-run failed jobs** on the same Actions run to reuse its checked artifacts and tag. The publisher verifies asset checksums before making the draft public.

From the GitHub CLI, select a mode explicitly.

```sh
gh workflow run pancake-release.yml --ref main -f mode=build-only
gh workflow run pancake-release.yml --ref main -f mode=prerelease
gh workflow run pancake-release.yml --ref main -f mode=stable
```

## Copilot usage

Copilot runs automatically when the workflow prepares a release candidate on the default branch. The workflow uses the repository owner's Copilot allowance through the built-in Actions token. Set your personal Copilot **Additional usage** budget to **$0** to prevent overage charges. [Copilot account settings](https://github.com/settings/copilot/features) · [Billing and usage](https://github.com/settings/billing)

Run **build-only** on the default branch and check the run summary for the notes source to confirm Copilot is working.
