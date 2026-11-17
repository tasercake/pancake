#!/usr/bin/env python3
"""Make an independent guardrail crate with the actual workspace lint policy."""

from pathlib import Path
import sys
import tomllib

from dependency_policy import toml_value


def main():
    """Copy lint/profile rules, not application dependencies or vendor patches."""
    try:
        source = tomllib.loads(Path(sys.argv[1]).read_text())
        # Cargo metadata must stay valid even after the application gains custom
        # binary targets, build scripts, features, dependencies and source patches.
        package_fields = {"name", "version", "edition", "rust-version", "description", "license",
                          "repository", "homepage", "readme", "keywords", "categories", "publish"}
        package = {key: value for key, value in source["package"].items() if key in package_fields}
        manifest = {
            "package": package,
            "workspace": {"members": ["."], "resolver": "3", "lints": source["workspace"]["lints"]},
            "lints": {"workspace": True},
            "profile": source.get("profile", {}),
        }
        output = "# Independent fixture; exact production lint policy.\n"
        for section, values in manifest.items():
            output += f"\n[{section}]\n"
            output += "".join(f"{key} = {toml_value(value)}\n" for key, value in values.items())
        tomllib.loads(output)
        Path(sys.argv[2]).write_text(output)
    except (OSError, ValueError, KeyError, IndexError, TypeError) as error:
        print(f"Guardrail manifest: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
