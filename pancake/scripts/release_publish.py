"""Stage and publish exactly the checked Pancake assets and source commit."""

import argparse
import json
import os
import subprocess
from pathlib import Path
from urllib.parse import urlencode, urlsplit
from urllib.request import Request, urlopen

from package_notices import file_sha256
from release_support import release_version

SOURCE = Path(__file__).resolve().parents[1]
CHECKSUMS = "SHA256SUMS.txt"


def git(*args):
    """Run Git in the checked release candidate."""
    return subprocess.check_output(["git", *args], cwd=SOURCE, text=True).strip()


def github(*args, payload=None):
    """Run the authenticated GitHub CLI and send optional JSON on standard input."""
    if payload is None:
        return subprocess.check_output(["gh", *args], cwd=SOURCE, text=True)
    return subprocess.check_output(
        ["gh", *args, "--input", "-"], cwd=SOURCE, text=True, input=json.dumps(payload),
    )


def upload_asset(upload_url, path):
    """Stream a packaged file to the upload URL returned for this release."""
    url = upload_url.partition("{")[0]
    parts = urlsplit(url)
    if parts.scheme != "https" or parts.netloc != "uploads.github.com" or parts.query or parts.fragment:
        raise ValueError("GitHub returned an unexpected release asset upload URL")
    query = urlencode({"name": path.name})
    with path.open("rb") as stream:
        request = Request(f"{url}?{query}", data=stream, method="POST", headers={
            "Accept": "application/vnd.github+json",
            "Content-Type": "application/octet-stream",
            "Content-Length": str(os.fstat(stream.fileno()).st_size),
            "X-GitHub-Api-Version": "2026-03-10",
        })
        request.add_unredirected_header("Authorization", f"Bearer {os.environ['GH_TOKEN']}")
        with urlopen(request, timeout=120) as response:
            return json.load(response)


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
        + "- **Apple Silicon** — Open the DMG and drag **Pancake Stacker** into **Applications**. "
        + "This build uses ad-hoc signing. "
        + "If macOS blocks the first launch, go to **System Settings → Privacy & Security**, "
        + "scroll to **Security**, and click **Open Anyway**. The DMG includes **First Launch Help** "
        + "with a Settings link. [Apple's illustrated instructions](https://support.apple.com/en-us/102445)\n"
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
    changelog = (candidate / "changelog.md").read_text(encoding="utf-8")
    notes = release_notes(repo, revision, version, changelog)
    tag = f"v{version}"
    tag_ref = f"refs/tags/{tag}"
    if current := git("ls-remote", "--tags", "origin", tag_ref):
        if current.split()[0] != revision:
            raise ValueError(f"{tag} now points to a different commit")
    else:
        git("push", "origin", f"{revision}:{tag_ref}")

    releases = f"repos/{repo}/releases"
    pages = json.loads(github("api", "--paginate", "--slurp", f"{releases}?per_page=100"))
    existing = [release for page in pages for release in page if release["tag_name"] == tag]
    if len(existing) > 1:
        raise ValueError(f"Multiple GitHub releases use {tag}")
    if existing:
        release = existing[0]
    else:
        # GitHub's release list can lag behind draft creation. Use the returned ID and upload URL.
        release = json.loads(github("api", "--method", "POST", releases, payload={
            "tag_name": tag, "target_commitish": revision, "name": f"Pancake Stacker {tag}",
            "body": notes, "draft": True, "prerelease": prerelease,
        }))
    if (release["tag_name"] != tag or release["target_commitish"] != revision
            or release["prerelease"] != prerelease):
        raise ValueError(f"GitHub release {tag} targets a different tag, commit or channel")

    endpoint = f"{releases}/{release['id']}"
    assets = json.loads(github("api", f"{endpoint}/assets?per_page=100")) if existing else release["assets"]
    missing = missing_assets(expected, assets)
    if release["draft"]:
        for name in missing:
            asset = upload_asset(release["upload_url"], distribution / name)
            missing_assets({name: expected[name]}, [asset])
        release = json.loads(github("api", "--method", "PATCH", endpoint, payload={
            "draft": False, "body": notes, "make_latest": "false" if prerelease else "true",
        }))
    elif missing:
        raise ValueError(f"Published release {tag} is missing verified assets")
    if release["draft"]:
        raise ValueError(f"GitHub release {tag} is still a draft")
    print(release["html_url"])


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
