"""Portable build entry point: no dependency on one developer's drives."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('vba8_build', ROOT / 'build.py')

class BuildTests(unittest.TestCase):
    def module(self):
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_import_has_no_build_side_effects(self):
        module = self.module()
        self.assertTrue(callable(module.main))

    def test_toolchain_uses_path_not_a_fixed_drive(self):
        module = self.module()
        tools = module.resolve_tools({}, lambda name: {'cargo': 'X:/rust/cargo.exe', 'npm': 'X:/node/npm.cmd'}.get(name))
        self.assertEqual(tools, ('X:/rust/cargo.exe', 'X:/node/npm.cmd'))

    def test_explicit_overrides_take_precedence(self):
        module = self.module()
        self.assertEqual(module.resolve_tools({'VBA8_CARGO': 'Z:/cargo.exe', 'VBA8_NPM': 'Z:/npm.cmd'}, lambda name: None), ('Z:/cargo.exe', 'Z:/npm.cmd'))

    def test_missing_tool_has_actionable_error(self):
        module = self.module()
        with self.assertRaisesRegex(RuntimeError, 'cargo'):
            module.resolve_tools({}, lambda name: None)

if __name__ == '__main__':
    unittest.main()
