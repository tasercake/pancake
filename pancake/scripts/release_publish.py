"""Stage and publish exactly the checked Pancake assets and source commit."""

import argparse
import json
import os
import subprocess
from pathlib import Path

from package_notices import file_sha256
from release_support import release_version

SOURCE = Path(__file__).resolve().parents[1]
CHECKSUMS = "SHA256SUMS.txt"


def git(*args):
    """Run Git in the checked release candidate."""
    return subprocess.check_output(["git", *args], cwd=SOURCE, text=True).strip()


def github(*args):
    """Run the authenticated GitHub CLI for the workflow repository."""
    return subprocess.check_output(["gh", *args], cwd=SOURCE, text=True)


def missing_assets(expected, assets):
    """Verify every existing asset and report which expected files need upload."""
    seen = set()
    for asset in assets:
        name = asset["name"]
        if name in seen or name not in expected:
            raise ValueError(f"Unexpected release asset: {name}")
        if asset["state"] != "uploaded" or asset["digest"] != expected[name]:
            raise ValueError(f"Unexpected release asset digest or upload state: {name}")
        seen.add(name)
    return [name for name in expected if name not in seen]


def package_names(version):
    """Return the same platform filenames for the downloads and the upload check."""
    return (
        f"pancake_{version}_amd64.deb",
        f"pancake-{version}-macos-arm64.dmg",
        f"pancake-gui-{version}-windows-x64.exe",
        f"pancake-cli-{version}-windows-x64.exe",
        f"pancake-notices-{version}-windows-x64.tar.gz",
    )


def release_notes(repository, revision, version, changelog):
    """Put direct download links first, followed by changes and installation help."""
    deb, mac, windows_gui, windows_cli, windows_notices = package_names(version)
    repo_url = f"https://github.com/{repository}"
    download = f"{repo_url}/releases/download/v{version}"
    return (
        "## Downloads\n\n"
        "| Platform | Download |\n"
        "| --- | --- |\n"
        f"| macOS (Apple Silicon) | [Disk image (.dmg)]({download}/{mac}) |\n"
        f"| Windows x64 | [Desktop app (.exe)]({download}/{windows_gui}) · "
        f"[CLI (.exe)]({download}/{windows_cli}) · [License notices (.tar.gz)]({download}/{windows_notices}) |\n"
        f"| Ubuntu 24.04 x86-64 | [Debian package (.deb)]({download}/{deb}) |\n\n"
        f"[SHA-256 checksums]({download}/{CHECKSUMS})\n\n"
        + changelog.rstrip()
        + "\n\n## Installation\n\n"
        + "- **Apple Silicon** — Open the DMG and drag Pancake.app into Applications. "
        + "The app is ad-hoc signed and has no Apple notarization. The image also includes pancake-cli.\n"
        + f"- **Ubuntu 24.04 x86-64** — Install the DEB with `sudo apt install ./{deb}`.\n"
        + "- **Windows x64** — Run the standalone desktop or CLI executable. Both are unsigned.\n\n"
        + f"[Installation and first-launch help]({repo_url}/blob/{revision}/pancake/docs/install.md) · "
        + f"[Stack your first photos]({repo_url}/blob/{revision}/pancake/docs/desktop.md) · "
        + f"[Source and build instructions]({repo_url}/tree/{revision}#build-from-source)\n"
    )


def publish(distribution, candidate, prerelease):
    """Bind the tag to the candidate and publish only after verifying draft contents."""
    repo = os.environ["GH_REPO"]
    revision = git("rev-parse", "HEAD")
    version = release_version(SOURCE)
    packages = package_names(version)
    expected = {name: f"sha256:{file_sha256(distribution / name)}" for name in packages}
    checksums = distribution / CHECKSUMS
    checksums.write_text("".join(f"{expected[name][7:]}  {name}\n" for name in packages), encoding="utf-8")
    expected[checksums.name] = f"sha256:{file_sha256(checksums)}"
    notes = candidate / "release-notes.md"
    changelog = (candidate / "changelog.md").read_text(encoding="utf-8")
    notes.write_text(release_notes(repo, revision, version, changelog), encoding="utf-8")
    tag = f"v{version}"
    tag_ref = f"refs/tags/{tag}"
    if current := git("ls-remote", "--tags", "origin", tag_ref):
        if current.split()[0] != revision:
            raise ValueError(f"{tag} now points to a different commit")
    else:
        git("push", "origin", f"{revision}:{tag_ref}")

    pages = json.loads(github("api", "--paginate", "--slurp", f"repos/{repo}/releases?per_page=100"))
    existing = [release for page in pages for release in page if release["tag_name"] == tag]
    if len(existing) > 1:
        raise ValueError(f"Multiple GitHub releases use {tag}")
    if not existing:
        flags = ["--prerelease"] if prerelease else []
        github("release", "create", tag, "--verify-tag", "--draft", "--target", revision,
               "--title", f"Pancake Stacker {tag}", "--notes-file", str(notes), *flags)
        pages = json.loads(github("api", "--paginate", "--slurp", f"repos/{repo}/releases?per_page=100"))
        existing = [release for page in pages for release in page if release["tag_name"] == tag]
    if len(existing) != 1:
        raise ValueError(f"GitHub did not return the staged release for {tag}")
    release = existing[0]
    if release["target_commitish"] != revision or release["prerelease"] != prerelease:
        raise ValueError(f"GitHub release {tag} targets a different commit or channel")
    endpoint = f"repos/{repo}/releases/{release['id']}/assets?per_page=100"
    assets = json.loads(github("api", endpoint))
    missing = missing_assets(expected, assets)
    if release["draft"]:
        if missing:
            github("release", "upload", tag, *(str(distribution / name) for name in missing))
        assets = json.loads(github("api", endpoint))
        if missing_assets(expected, assets):
            raise ValueError(f"GitHub release {tag} is still missing assets")
        latest = "--latest=false" if prerelease else "--latest"
        github("release", "edit", tag, "--draft=false", "--notes-file", str(notes), latest)
    elif missing:
        raise ValueError(f"Published release {tag} is missing verified assets")
    details = json.loads(github("api", f"repos/{repo}/releases/{release['id']}"))
    if details["draft"]:
        raise ValueError(f"GitHub release {tag} is still a draft")
    print(details["html_url"])


def main():
    """Publish the candidate checked out by the workflow."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("distribution", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--prerelease", action="store_true")
    args = parser.parse_args()
    publish(args.distribution, args.candidate, args.prerelease)


if __name__ == "__main__":
    main()
