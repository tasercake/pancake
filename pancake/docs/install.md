# Install Pancake Stacker

Choose the newest release and expand **Assets** to find the download for your computer. For releases that offer ZIP downloads, follow the installation instructions linked from the release notes. [Pancake Stacker releases](https://github.com/tasercake/pancake/releases)

## macOS (Apple Silicon)

1. Download `pancake-<version>-macos-arm64.dmg` and double-click it in Finder.
2. In the **Pancake Stacker** Finder window, drag **Pancake Stacker** to the **Applications** shortcut. You can also copy the app into your user **Applications** folder.
3. Open Pancake Stacker from **Applications**. Eject the disk image in Finder when you're done with it.

To tell whether your Mac uses Apple Silicon, open **Apple menu → About This Mac** and look for an Apple chip.

The app is ad-hoc signed and has no Apple notarization. If macOS blocks the first launch, check that you downloaded from the project's Releases page, then follow [Apple's instructions for opening the app](https://support.apple.com/en-us/102445) under **System Settings → Privacy & Security**.

## Windows x64

Download `pancake-gui-<version>-windows-x64.exe` and double-click it in File Explorer. You can move the executable to any folder you want to keep it in.

The command-line tool is available as `pancake-cli-<version>-windows-x64.exe`. Licenses and dependency notices are available in `pancake-notices-<version>-windows-x64.tar.gz`.

To check your architecture, open **Settings → System → About** and look for an x64-based processor under **System type**.

The executables are unsigned, so Windows may show a warning. If Pancake Stacker reports a missing Visual C++ runtime DLL, install the [Microsoft Visual C++ Redistributable for x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist#latest-supported-redistributable-version), then try again.

## Ubuntu 24.04 x86-64

1. Download `pancake_<version>_amd64.deb` to **Downloads**.
2. Open Terminal and run the commands below. Replace `0.1.0` with the version in your downloaded filename.

   ```sh
   cd ~/Downloads
   sudo apt install ./pancake_0.1.0_amd64.deb
   ```

3. Open **Pancake Stacker** from the application menu.

Once Pancake Stacker opens, [stack your first photos](desktop.md#make-a-focus-stack).
