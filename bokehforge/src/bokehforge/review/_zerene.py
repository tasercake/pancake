"""Local Zerene Stacker Batch API shared by Review and the research runner."""

from codecs import BOM_UTF16_BE, BOM_UTF16_LE
from contextlib import contextmanager
import os
from pathlib import Path
import platform
import shlex
import xml.etree.ElementTree as ET

MAIN_CLASS = "com.zerenesystems.stacker.gui.MainFrame"
TASK_CODES = {"pmax": "1", "dmap": "2"}
INSTALL_HINT = "Install Zerene Stacker on this computer, open it once with an active trial or license, then quit it."


def supported():
    return platform.system() in {"Darwin", "Windows"}


def preferences_directory():
    system = platform.system()
    if system == "Darwin":
        return Path.home() / "Library/Preferences/ZereneStacker"
    if system == "Windows" and (appdata := os.environ.get("APPDATA")):
        return Path(appdata) / "ZereneStacker"
    if system == "Windows":
        raise ValueError("Run Review from a Windows desktop session with APPDATA set.")
    raise ValueError("Zerene Stacker evaluation requires a macOS or Windows Review server.")


def _windows_argv(recorded):
    import ctypes
    from ctypes import wintypes

    # Zerene records a Windows command line. POSIX shlex would consume path backslashes.
    split = ctypes.windll.shell32.CommandLineToArgvW
    split.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(ctypes.c_int)]
    split.restype = ctypes.POINTER(wintypes.LPWSTR)
    free = ctypes.windll.kernel32.LocalFree
    free.argtypes = [wintypes.HLOCAL]
    free.restype = wintypes.HLOCAL
    count = ctypes.c_int()
    argv = split(recorded, ctypes.byref(count))
    if not argv:
        raise ValueError("Could not read the Zerene launch command.")
    try:
        return list(argv[:count.value])
    finally:
        free(ctypes.cast(argv, wintypes.HLOCAL))


def launch_command(preferences):
    recorded = preferences / "zerenstk.launchcmd"
    if not recorded.is_file():
        raise ValueError(INSTALL_HINT)
    windows = platform.system() == "Windows"
    data = recorded.read_bytes()
    encoding = "utf-16" if data.startswith((BOM_UTF16_LE, BOM_UTF16_BE)) else "utf-8-sig"
    try:
        text = data.decode(encoding)
    except UnicodeDecodeError:
        if not windows:
            raise
        text = data.decode("mbcs")
    command = _windows_argv(text.strip()) if windows else shlex.split(text)
    if not command or MAIN_CLASS not in command:
        raise ValueError(f"Invalid Zerene launch command. {INSTALL_HINT}")
    java = Path(command[0])
    if java.name.lower() == "javaw.exe":
        java = java.with_name("java.exe")
    if (java.name.lower() not in {"java", "java.exe"} or not java.is_absolute()
            or str(java).startswith(("//", "\\")) or not java.is_file()):
        raise ValueError(f"Could not find Zerene's local Java runtime. {INSTALL_HINT}")
    # The recorded launch can include unrelated temporary inputs and flags.
    return [str(java), *command[1:command.index(MAIN_CLASS) + 1]]


def make_batch(source, output, methods, *, align, dmap_percentile,
               dmap_estimation_radius, dmap_smoothing_radius):
    batch = ET.Element("Batch")
    sources = ET.SubElement(batch, "Sources", length="1")
    ET.SubElement(sources, "Source", value=str(source))
    ET.SubElement(batch, "ProjectDispositionCode", value="101")  # Temporary project
    tasks = ET.SubElement(batch, "Tasks", length=str(len(methods)))
    for method in methods:
        task = ET.SubElement(tasks, "Task")
        ET.SubElement(task, "OutputImageDispositionCode", value="4")  # Designated folder
        ET.SubElement(task, "OutputImagesDesignatedFolder", value=str(output))
        preferences = ET.SubElement(task, "Preferences")
        values = {
            "OutputImageNaming.Template": method,
            "AlignmentControl.AddNewFilesAsAlreadyAligned": str(not align).lower(),
            "SaveImage.FileType": "tif",
            "SaveImage.BitsPerColor": "16",
            "UIStyle.ShowIntermediateOutputInterval": "0.0",
        }
        if method == "pmax":
            values.update({
                "PyramidControl.RetainUDRImage": "false",
                "PyramidControl.RetainOnlyUDRImage": "false",
            })
        else:
            values.update({
                "DepthMapControl.UseFixedContrastThresholdLevel": "false",
                "DepthMapControl.UseFixedContrastThresholdPercentile": "true",
                "DepthMapControl.ContrastThresholdPercentile": str(dmap_percentile),
                "DepthMapControl.EstimationRadius": str(dmap_estimation_radius),
                "DepthMapControl.SmoothingRadius": str(dmap_smoothing_radius),
                "DepthMapControl.SaveDepthMapImage": "false",
            })
        for key, value in values.items():
            ET.SubElement(preferences, key, value=value)
        ET.SubElement(task, "TaskIndicatorCode", value=TASK_CODES[method])
    return batch


def write_batches(path, batches):
    root = ET.Element("ZereneStackerBatchScript")
    queue = ET.SubElement(root, "BatchQueue")
    ET.SubElement(queue, "Batches", length=str(len(batches))).extend(batches)
    ET.indent(root)
    ET.ElementTree(root).write(path, encoding="UTF-8", xml_declaration=True)


def write_batch(path, output, methods, **options):
    write_batches(path, [make_batch("%CurrentProject%", output, methods, **options)])


def batch_command(java, batch_path, inputs=()):
    background = ["-Dapple.awt.BackgroundOnly=true"] if platform.system() == "Darwin" else []
    return [
        *java[:-1], *background, java[-1],
        "-noSplashScreen", "-runMinimized", "-showProgressWhenMinimized=false",
        "-suppressSaveProperties", "-exitOnBatchScriptCompletion",
        "-batchScript", str(batch_path), *map(str, inputs),
    ]


@contextmanager
def batch_lock(preferences):
    with (preferences / "batch-runner.lock").open("a+b") as lock:
        try:
            if os.name == "nt":
                import msvcrt
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            raise ValueError("Another Zerene batch runner is active.") from error
        yield


def batch_output(output, method):
    files = list(output.glob(f"*_{method}.tif"))
    if len(files) != 1:
        raise ValueError(f"Expected one Zerene {method} TIFF, found {len(files)}. See the stacker log.")
    return files[0]
