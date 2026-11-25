"""Prepare known evaluation tools without modifying system packages."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import platform
import re
from shutil import which
import signal
import subprocess
from tempfile import TemporaryDirectory
from time import monotonic
from urllib.request import urlopen
from zipfile import ZipFile

REPO = Path(__file__).resolve().parents[4]
BIN_DIR = "Scripts" if os.name == "nt" else "bin"
EXE = ".exe" if os.name == "nt" else ""

SUBMODULES = {
    "focus-stack": (".submodules/focus-stack", "https://github.com/PetteriAimonen/focus-stack.git"),
    "openfocus": (".submodules/OpenFocus", "https://github.com/Xinzhe99/OpenFocus.git"),
    "shinestacker": (".submodules/shinestacker", "https://github.com/lucalista/shinestacker.git"),
}

# Release 1.5 is the pinned gitlink. These are the publisher's recorded SHA-256 digests.
FOCUS_STACK_PIN = "0cd289e224fe24aead5b146faeaa46950775f346"
FOCUS_STACK_RELEASES = {
    ("Linux", "x86_64"): ("focus-stack.AppImage",
                         "97740c829bbd1d9f873c09694a58f22944c9ec8237d3a30360877960b3bff865",
                         "squashfs-root/AppRun"),
    ("Darwin", "arm64"): ("focus-stack_MacOSX.zip",
                          "332b0667c0edd92c75c37b56476c46ca2aa729bb2aa63476bb5704776300ea96",
                          "focus-stack/focus-stack.app/Contents/MacOS/focus-stack"),
    ("Windows", "amd64"): ("focus-stack_Windows.zip",
                           "0104c863e1fc961cd87520c87d41c927d2968822d48fd37e76da00e1dfb6c7c9",
                           "focus-stack/focus-stack.exe"),
}


class RunStopped(Exception):
    pass


def executable(*candidates):
    for candidate in candidates:
        if candidate and (found := which(str(candidate))):
            # Resolving a venv's Python symlink would discard its environment.
            return str(Path(found).absolute())
    return None


def _require(name, hint):
    if found := executable(name):
        return found
    raise RuntimeError(f"{name} is required for automatic setup. {hint}")


def _terminate(process):
    if os.name == "nt":
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, check=False, timeout=5)
    else:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        if os.name != "nt":
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
        process.wait()


def run_logged(command, cwd, log, stop, action, *, timeout=None, env=None):
    if stop.is_set():
        raise RunStopped()
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("ab") as stream:
        stream.write((json.dumps(command) + "\n").encode())
        stream.flush()
        process = subprocess.Popen(command, cwd=cwd, stdin=subprocess.DEVNULL,
                                   stdout=stream, stderr=subprocess.STDOUT,
                                   start_new_session=os.name != "nt", env=env)
        deadline = monotonic() + timeout if timeout else None
        while process.poll() is None:
            if stop.wait(0.25):
                _terminate(process)
                raise RunStopped()
            if deadline is not None and monotonic() >= deadline:
                _terminate(process)
                raise RuntimeError(f"{action} timed out. See the stacker log.")
        if process.returncode:
            raise RuntimeError(f"{action} exited with code {process.returncode}. See the stacker log.")


def _download(url, destination, checksum, stop):
    with urlopen(url, timeout=20) as remote, destination.open("wb") as local:
        while chunk := remote.read(1 << 20):
            if stop.is_set():
                raise RunStopped()
            local.write(chunk)
    with destination.open("rb") as local:
        if file_digest(local, "sha256").hexdigest() != checksum:
            raise RuntimeError("The publisher's release checksum did not match the download")


class StackerSetup:
    def __init__(self, store, repo=REPO):
        self.root = store / "tools"
        self.repo = repo
        self.pins = self._read_pins()

    def _read_pins(self):
        if not (self.repo / ".gitmodules").is_file() or not executable("git"):
            return {}
        try:
            data = subprocess.check_output(["git", "ls-tree", "HEAD", "--", *(
                path for path, _ in SUBMODULES.values())], cwd=self.repo, text=True, timeout=5)
        except (OSError, subprocess.SubprocessError):
            return {}
        return {path: revision for revision, path in
                re.findall(r"^160000 commit ([0-9a-f]+)\t(.+)$", data, re.MULTILINE)}

    def _directory(self, stacker):
        if stacker == "pancake":
            return self.repo / "pancake/target"
        pin = self.pins.get(SUBMODULES[stacker][0])
        return self.root / stacker / pin if pin else None

    def available(self, tool):
        if configured := executable(*tool.candidates):
            return configured
        directory = self._directory(tool.id)
        # The provisioner only publishes a path after its tool smoke check succeeds.
        receipt = directory / "ready" if directory is not None and tool.id != "pancake" else None
        if receipt is not None and receipt.is_file():
            return executable(directory / receipt.read_text(encoding="utf-8").strip())
        return None

    def _checkout(self, stacker_id, directory, log, stop):
        path, origin = SUBMODULES[stacker_id]
        git = _require("git", "Install Git and run Review from the repository checkout.")
        pin = self.pins.get(path)
        if not pin:
            raise RuntimeError(f"Could not read the pinned source for {stacker_id}. Run Review from the repository checkout.")
        local = self.repo / path
        # Use an initialized local submodule if it has the pinned commit.
        if (local / ".git").exists():
            try:
                subprocess.run([git, "cat-file", "-e", f"{pin}^{{commit}}"], cwd=local, check=True,
                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, timeout=5)
                origin = str(local)
            except (OSError, subprocess.SubprocessError):
                pass
        run_logged([git, "init", "--quiet"], directory, log, stop, "Initializing the source directory")
        run_logged([git, "fetch", "--no-tags", "--depth=1", origin, pin],
                   directory, log, stop, "Fetching the pinned stacker source")
        run_logged([git, "checkout", "--quiet", "--detach", "FETCH_HEAD"], directory,
                   log, stop, "Checking out the pinned stacker source")

    def _pancake(self, log, stop, tell):
        cargo = _require("cargo", "Install Rust with rustup for this host.")
        work = self.repo / "pancake"
        if not (work / "Cargo.toml").is_file():
            raise RuntimeError("Run Review from the Pancake repository to build Pancake")
        tell("Building Pancake with its pinned toolchain")
        run_logged([cargo, "build", "--locked", "--release", "--package", "pancake-cli",
                    "--target-dir", str(work / "target")], work, log, stop, "Building Pancake")
        return work / "target/release" / ("pancake-cli" + EXE)

    def _python_stacker(self, tool, directory, log, stop, tell):
        uv = _require("uv", "Install uv or point Review at an existing stacker.")
        venv = directory / ".venv"
        python = venv / BIN_DIR / ("python" + EXE)
        with TemporaryDirectory(prefix="source-", dir=directory) as scratch:
            scratch = Path(scratch)
            tell("Fetching the pinned source")
            self._checkout(tool.id, scratch, log, stop)
            tell("Installing the Python environment and dependencies")
            run_logged([uv, "venv", "--allow-existing", "--python", "3.12", str(venv)],
                       directory, log, stop, "Creating the stacker environment")
            run_logged([uv, "pip", "install", "--python", str(python), str(scratch)],
                       directory, log, stop, "Installing the stacker dependencies")
        return python if tool.id == "shinestacker" else venv / BIN_DIR / ("openfocus" + EXE)

    def _focus_stack_release(self, asset, directory, scratch, log, stop, tell):
        filename, checksum, binary = asset
        archive = scratch / filename
        tell("Downloading and verifying the publisher’s OpenCV bundle")
        _download("https://github.com/PetteriAimonen/focus-stack/releases/download/1.5/" + filename,
                  archive, checksum, stop)
        tell("Unpacking focus-stack")
        if filename.endswith(".AppImage"):
            if unsquashfs := executable("unsquashfs"):
                # This offset is for the checksum-pinned AppImage above.
                run_logged([unsquashfs, "-no-progress", "-offset", "944632", str(archive)],
                           scratch, log, stop, "Unpacking the AppImage")
            else:
                archive.chmod(0o700)
                run_logged([str(archive), "--appimage-extract"], scratch, log, stop,
                           "Unpacking the AppImage (squashfs-tools can also unpack it)",
                           timeout=90, env={**os.environ, "APPIMAGELAUNCHER_DISABLE": "1"})
        elif platform.system() == "Darwin":
            run_logged(["/usr/bin/ditto", "-x", "-k", str(archive), str(scratch)],
                       scratch, log, stop, "Unpacking the macOS release")
        else:
            with ZipFile(archive) as release:
                release.extractall(scratch)
        relative = Path(binary)
        # Move the distribution as a unit so relative library locations stay intact.
        source = scratch / relative.parts[0]
        installed = directory / relative.parts[0]
        if installed.exists():
            installed = directory / f"{relative.parts[0]}-{scratch.name}"
        source.rename(installed)
        return installed.joinpath(*relative.parts[1:])

    def _focus_stack(self, directory, log, stop, tell):
        release = FOCUS_STACK_RELEASES.get((platform.system(), platform.machine().lower()))
        with TemporaryDirectory(prefix="source-", dir=directory) as scratch:
            scratch = Path(scratch)
            if release and self.pins.get(SUBMODULES["focus-stack"][0]) == FOCUS_STACK_PIN:
                return self._focus_stack_release(release, directory, scratch, log, stop, tell)
            make = _require("make", "Install the platform's C++ build tools and OpenCV development files.")
            tell("Building the pinned source against system OpenCV")
            self._checkout("focus-stack", scratch, log, stop)
            run_logged([make, "build/focus-stack"], scratch, log, stop, "Building focus-stack with OpenCV")
            binary = directory / ("focus-stack" + EXE)
            (scratch / "build/focus-stack").replace(binary)
            return binary

    def prepare(self, tool, log, stop, tell):
        if found := self.available(tool):
            return found
        if tool.id == "pancake":
            binary = self._pancake(log, stop, tell)
        else:
            directory = self._directory(tool.id)
            if directory is None:
                raise RuntimeError(f"Could not read the pinned {tool.label} source. Run Review from the repository checkout with Git installed.")
            directory.mkdir(parents=True, exist_ok=True)
            if tool.id == "focus-stack":
                binary = self._focus_stack(directory, log, stop, tell)
            else:
                binary = self._python_stacker(tool, directory, log, stop, tell)
        tell("Checking the installation")
        check = [str(binary), "--help" if tool.id == "openfocus" else "--version"]
        if tool.id == "shinestacker":
            # Import the same modules as the headless runner without launching a stack.
            check = [str(binary), "-c", (
                "from shinestacker.algorithms import AlignFrames, CombinedActions, FocusStack, PyramidStack, StackJob; "
                "from shinestacker.config.config import config")]
        run_logged(check, self.repo, log, stop, f"Checking {tool.label}", timeout=60)
        if tool.id != "pancake":
            pending = directory / "ready.tmp"
            pending.write_text(binary.relative_to(directory).as_posix(), encoding="utf-8")
            pending.replace(directory / "ready")
        return str(binary)
