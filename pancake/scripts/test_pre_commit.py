"""Exercise the real commit hook without allowing compiler or test commands."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

HOOK = Path(__file__).resolve().parents[1] / '.githooks/pre-commit'


class CommitHookTests(unittest.TestCase):
    """Keep commits cheap and preserve failure reporting and staging rules."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.git('init', '-q')
        for directory in ('pancake/.githooks', 'pancake/scripts', 'bin'):
            (self.root / directory).mkdir(parents=True)
        shutil.copyfile(HOOK, self.root / 'pancake/.githooks/pre-commit')
        (self.root / 'pancake/scripts/check.sh').write_text('echo forbidden-slow-check >&2; exit 99\n')
        self.git('add', 'pancake')
        cargo = self.root / 'bin/cargo'
        cargo.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$HOOK_LOG"\n'
                         '[ "$1" = fmt ] || exit 99\nexit "${FMT_RESULT:-0}"\n')
        cargo.chmod(0o755)
        self.environment = dict(os.environ, PATH=str(self.root / 'bin') + ':' + os.environ['PATH'],
                                HOOK_LOG=str(self.root / 'calls'))

    def git(self, *args):
        return subprocess.run(['git', *args], cwd=self.root, check=True, capture_output=True)

    def stage(self, name, content):
        (self.root / name).parent.mkdir(parents=True, exist_ok=True)
        (self.root / name).write_text(content)
        self.git('add', name)

    def run_hook(self, **environment):
        return subprocess.run(['bash', 'pancake/.githooks/pre-commit'], cwd=self.root,
                              env=dict(self.environment, **environment), capture_output=True, text=True)

    def test_documentation_needs_no_cargo(self):
        self.stage('pancake/README.md', 'Documentation\n')
        result = self.run_hook()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / 'calls').exists())

    def test_rust_runs_formatter_only(self):
        self.stage('pancake/example.rs', 'fn main() {}\n')
        result = self.run_hook()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'calls').read_text().split()[0], 'fmt')
        self.assertIn('--check', (self.root / 'calls').read_text())

    def test_nested_rust_and_manifest_trigger_formatting(self):
        for name in ('pancake/src/nested/file.rs', 'pancake/Cargo.toml'):
            with self.subTest(name=name):
                self.stage(name, '# fixture\n')
                (self.root / 'calls').unlink(missing_ok=True)
                result = self.run_hook()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((self.root / 'calls').read_text().split()[0], 'fmt')

    def test_format_failure_blocks_commit(self):
        self.stage('pancake/example.rs', 'fn main() {}\n')
        self.assertEqual(self.run_hook(FMT_RESULT='7').returncode, 7)

    def test_whitespace_failure_blocks_commit(self):
        self.stage('pancake/README.md', 'Trailing whitespace  \n')
        self.assertNotEqual(self.run_hook().returncode, 0)

    def test_partial_staging_is_rejected(self):
        self.stage('pancake/README.md', 'Staged\n')
        (self.root / 'pancake/README.md').write_text('Unstaged\n')
        self.assertNotEqual(self.run_hook().returncode, 0)


if __name__ == '__main__':
    unittest.main()
