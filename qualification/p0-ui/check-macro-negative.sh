#!/usr/bin/env bash
# Reproduce the selected generated-glue boundary failure without changing policy.
set -euo pipefail
probe=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$probe/../.." && pwd)
fixture="$root/artifacts/p0-ui-macro-negative"
mkdir -p "$fixture/src"
python3 - "$probe" "$fixture" <<'PY'
from pathlib import Path
import sys
source, destination = map(Path, sys.argv[1:])
manifest = (source / 'Cargo.toml').read_text()
start = manifest.index('[dependencies]')
end = manifest.index('[patch.crates-io]')
slint = next(line for line in manifest.splitlines() if line.startswith('slint = '))
manifest = manifest[:start] + '[dependencies]\n' + slint + '\n\n' + manifest[end:]
manifest = manifest.replace('name = "pancake-ui-qualification"', 'name = "pancake-ui-macro-negative"')
(destination / 'Cargo.toml').write_text(manifest)
(destination / 'src/main.rs').write_text((source / 'probes/macro_forbid.rs').read_text())
PY
if cargo +1.100.0 check --manifest-path "$fixture/Cargo.toml" > "$fixture/result.log" 2>&1; then
    printf 'Expected generated Slint glue to fail unsafe-forbid, but compilation succeeded.\n' >&2
    exit 1
fi
python3 - "$fixture/result.log" <<'PY'
from pathlib import Path
import sys
log = Path(sys.argv[1]).read_text()
assert 'error[E0453]' in log, log
assert 'allow(unsafe_code) incompatible with previous forbid' in log, log
assert 'ItemTreeVTable_static' in log, log
print('Confirmed generated Slint glue is incompatible with first-party unsafe-forbid.')
PY
