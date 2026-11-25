# Install Pancake Stacker

Open [Pancake releases](https://github.com/tasercake/pancake/releases), choose the newest release, and expand **Assets**. Download the package for your computer. Each package includes the desktop app and the command-line tool.

## macOS (Apple Silicon)

1. Download `pancake-<version>-macos-arm64.zip`.
2. Double-click the ZIP in Finder to extract it.
3. Move `Pancake.app` to **Applications** or your user **Applications** folder. Open the app from there.

To tell whether your Mac uses Apple Silicon, open **Apple menu → About This Mac** and look for an Apple chip.

The app is ad-hoc signed and has no Apple notarization. If macOS blocks the first launch, check that you downloaded from the project's Releases page, then follow [Apple's instructions for opening the app](https://support.apple.com/en-us/102445) under **System Settings → Privacy & Security**.

## Windows x64

1. Download `pancake-<version>-windows-x64.zip`.
2. Right-click the ZIP and choose **Extract All**.
3. Open `pancake-gui.exe` from the extracted folder. You can keep that folder wherever you want to run Pancake.

To check your architecture, open **Settings → System → About** and look for an x64-based processor under **System type**.

The executables are unsigned, so Windows may show a warning. If Pancake reports a missing Visual C++ runtime DLL, install the [Microsoft Visual C++ Redistributable for x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist#latest-supported-redistributable-version), then try again.

## Ubuntu 24.04 x86-64

1. Download `pancake_<version>_amd64.deb` to **Downloads**.
2. Open Terminal and run the commands below. Replace `0.1.0` with the version in your downloaded filename.

   ```sh
   cd ~/Downloads
   sudo apt install ./pancake_0.1.0_amd64.deb
   ```

3. Open **Pancake** from the application menu.

Once Pancake opens, [stack your first photos](desktop.md#make-a-focus-stack).
