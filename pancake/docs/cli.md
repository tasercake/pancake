# Pancake command line

Get the command-line executable for your platform. [Pancake downloads](install.md)

- On Ubuntu, install the DEB and run `pancake-cli`.
- On macOS, install the app and run `/Applications/Pancake.app/Contents/MacOS/pancake-cli` in Terminal. Adjust the path if you installed the app elsewhere.
- On Windows, download `pancake-cli-<version>-windows-x64.exe` and run it from PowerShell in the download folder. Use `.\pancake-cli-<version>-windows-x64.exe` with the version in your downloaded filename, or `.\pancake-cli.exe` if you renamed it.

## Stack files

Use `--input` to list two or more files and `--output` to name the result. Quote paths that contain spaces. For example, this command uses the CPU and writes a 16-bit TIFF.

```sh
pancake-cli --cpu --input "near.tif" "far.tif" --output "stacked.tif" --bit-depth 16
```

For an entire series, you can pass a quoted glob such as `"frames/*.tif"`. The output extension selects the format.

## Choose settings

- Add `--align --auto-crop` to align shifts between frames and crop to the usable area.
- Add `--alignment-mode breathing` to account for changes in scale as focus moves.
- Use `--method exposure_fusion` for one exposure bracket shot at the same focus distance.
- Run `pancake-cli --help` for the full list of methods, alignment modes, device choices, and output options.
