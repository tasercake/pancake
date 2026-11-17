"""Guard the explicit cross-host Miri contract without weakening native gates."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class MiriHostPolicyTests(unittest.TestCase):
    """Changed source and an unavailable host must never become a Miri pass."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / 'src').mkdir()
        (self.root / 'scripts').mkdir()
        (self.root / 'scripts/tool-versions.sh').write_text('PANCAKE_NIGHTLY=nightly-2026-11-15\n')
        files = {'src/lib.rs': 'pub fn checked() {}\n', 'Cargo.toml': '[package]\n', 'Cargo.lock': 'version = 4\n'}
        for name, content in files.items():
            (self.root / name).write_text(content)
        self.policy = {'schema_version': 1, 'canonical_miri_host': 'Darwin',
                       'required_ci_job': 'nightly', 'nightly': 'nightly-2026-11-15',
                       'reviewed_files': {name: hashlib.sha256((self.root / name).read_bytes()).hexdigest() for name in files}}
        (self.root / 'assurance-platforms.json').write_text(json.dumps(self.policy))

    def invoke(self, host, mode):
        return subprocess.run([sys.executable, str(Path(__file__).with_name('miri_host_policy.py')),
                               str(self.root), mode, '--host', host], capture_output=True, text=True, check=False)

    def test_linux_all_keeps_native_gates_and_names_required_external_miri(self):
        result = self.invoke('Linux', 'all-plan')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ['fast', 'test', 'coverage', 'deps', 'mutants', 'layouts', 'branches', 'guardrails'])
        self.assertIn('Miri NOT RUN on Linux', result.stderr)
        self.assertIn('required macOS CI job: nightly', result.stderr)
        self.assertNotIn('PASS', result.stderr)

    def test_mac_all_keeps_actual_miri_and_linux_cannot_claim_local_miri(self):
        result = self.invoke('Darwin', 'all-plan')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('nightly', result.stdout.splitlines())
        refused = self.invoke('Linux', 'miri')
        self.assertNotEqual(refused.returncode, 0)
        self.assertIn('requires canonical Darwin host', refused.stderr)

    def test_changed_or_new_production_source_requires_boundary_review(self):
        (self.root / 'src/lib.rs').write_text('#[cfg(target_os = "linux")] pub fn changed() {}\n')
        result = self.invoke('Darwin', 'validate')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('changed reviewed file', result.stderr)
        (self.root / 'src/lib.rs').write_text('pub fn checked() {}\n')
        (self.root / 'src/linux.rs').write_text('pub fn platform_only() {}\n')
        result = self.invoke('Darwin', 'validate')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('production source inventory changed', result.stderr)

    def test_dependency_resolution_and_unknown_platform_fail_closed(self):
        (self.root / 'Cargo.lock').write_text('version = 5\n')
        result = self.invoke('Darwin', 'validate')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('changed reviewed file', result.stderr)
        (self.root / 'Cargo.lock').write_text('version = 4\n')
        (self.root / 'scripts/tool-versions.sh').write_text('PANCAKE_NIGHTLY=nightly-other\n')
        result = self.invoke('Darwin', 'validate')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('nightly tool pin changed', result.stderr)
        (self.root / 'scripts/tool-versions.sh').write_text('PANCAKE_NIGHTLY=nightly-2026-11-15\n')
        result = self.invoke('UnknownOS', 'all-plan')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('unclassified assurance host', result.stderr)


if __name__ == '__main__':
    unittest.main()
