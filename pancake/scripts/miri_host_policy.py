#!/usr/bin/env python3
"""Bind cross-host Miri qualification to reviewed OS-neutral production source."""

import argparse
import hashlib
import json
from pathlib import Path
import platform
import sys


def validate(root):
    """Reject changes to the source, dependency closure or canonical tool lane."""
    policy = json.loads((root / 'assurance-platforms.json').read_text())
    if (policy['schema_version'] != 1 or policy['canonical_miri_host'] != 'Darwin'
            or policy['required_ci_job'] != 'nightly'):
        raise ValueError('Miri platform contract changed; review the canonical lane')
    pins = (root / 'scripts/tool-versions.sh').read_text().splitlines()
    if [line.partition('=')[2] for line in pins if line.startswith('PANCAKE_NIGHTLY=')] != [policy['nightly']]:
        raise ValueError('nightly tool pin changed; review cross-host Miri boundary')
    reviewed = policy['reviewed_files']
    production = {str(path.relative_to(root)) for path in (root / 'src').rglob('*.rs')}
    if (root / 'build.rs').exists():
        production.add('build.rs')
    if set(reviewed) != production | {'Cargo.toml', 'Cargo.lock'}:
        raise ValueError('production source inventory changed; review cross-host Miri boundary')
    for name, expected in reviewed.items():
        if hashlib.sha256((root / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f'changed reviewed file {name}; review cross-host Miri boundary')
    return policy


def main():
    """Print local gates or fail if local execution would misstate Miri coverage."""
    parser = argparse.ArgumentParser()
    parser.add_argument('root', type=Path)
    parser.add_argument('mode', choices=['validate', 'all-plan', 'miri'])
    parser.add_argument('--host', default=platform.system(), help='Testable host classification; actual Miri is never simulated')
    args = parser.parse_args()
    try:
        policy = validate(args.root.resolve())
        if args.host not in ('Darwin', 'Linux'):
            raise ValueError(f'unclassified assurance host: {args.host}')
        if args.mode == 'miri':
            if args.host != policy['canonical_miri_host']:
                raise ValueError('Miri requires canonical Darwin host; Linux layout/branch checks and required macOS CI remain mandatory')
            print('Miri: executing required macOS pure-host tests; no Linux Miri pass is claimed.')
        elif args.mode == 'all-plan':
            if args.host == 'Darwin':
                gates = ['fast', 'test', 'coverage', 'deps', 'mutants', 'nightly', 'branches', 'guardrails']
                print('Miri: required local macOS execution in nightly gate.', file=sys.stderr)
            else:
                gates = ['fast', 'test', 'coverage', 'deps', 'mutants', 'layouts', 'branches', 'guardrails']
                print('Miri NOT RUN on Linux; required macOS CI job: nightly. Local all does not qualify the complete platform matrix.', file=sys.stderr)
            print('\n'.join(gates))
        else:
            print('Miri platform contract: reviewed production source and dependency closure match.')
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f'Miri platform contract: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
