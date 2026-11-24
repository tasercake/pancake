# Pancake Stacker

Open a series of photos taken at different focus distances, stack the frames, and export the result. See the [quickstart](../README.md#build-from-source) for desktop build instructions and a CLI example.

## Lightroom Classic on macOS

1. Move the packaged `Pancake.app` to Applications or your user's Applications folder.
2. Open Pancake, go to **Preferences**, and click **Install Lightroom plug-in**. Restart Lightroom Classic if it was open.
3. Select at least two photos in Lightroom Classic. Right-click the selection and open **Export**. Choose **Pancake (Originals)** for supported source files or **Pancake (With Lightroom Edits)** for full-size 16-bit sRGB TIFFs. Both commands are also in **File > Plug-in Extras**.
4. Stack and export the result from Pancake. TIFF export retains the embedded ICC profile when it is identical across all frames. To add the result to Lightroom, use Lightroom's Import command.

The installer copies the plug-in into Lightroom's user Modules folder and connects it to the installed app. If you move the app, open it from the new location and reinstall the plug-in from Preferences. Temporary TIFFs are removed when the Pancake instance that uses them closes.

## Development

See the [developer guide](DEVELOPING.md) for tests, CI, and assurance, and the [release guide](docs/releases.md) for packaging.
