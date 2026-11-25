# Pancake command line

The [Pancake packages](install.md) include the command-line executable. On Ubuntu, run `pancake-cli` after installing the package. On macOS or Windows, open a terminal in the extracted folder and use `./pancake-cli` or `.\pancake-cli.exe`, respectively.

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
