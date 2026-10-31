import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))


def test_guard_requires_real_executable_before_launch(tmp_path):
    from harness import validate_exe
    missing = tmp_path / 'vba8.exe'
    try:
        validate_exe(missing)
    except FileNotFoundError:
        pass
    else:
        raise AssertionError('missing executable accepted')
    wrong = tmp_path / 'other.exe'
    wrong.write_bytes(b'MZ')
    try:
        validate_exe(wrong)
    except ValueError:
        pass
    else:
        raise AssertionError('unrelated executable accepted')
    expected = tmp_path / 'vba8.exe'
    expected.write_bytes(b'MZ')
    assert validate_exe(expected) == expected.resolve()


def test_default_cli_does_not_start_or_import_automation():
    result = subprocess.run([sys.executable, str(ROOT / 'harness.py')], capture_output=True, text=True)
    assert result.returncode == 2
    assert 'usage:' in result.stderr.lower()


def test_preflight_is_readonly_and_reports_excel_registration():
    from harness import preflight
    report = preflight()
    assert report['excel']['clsid'] == '{00024500-0000-0000-C000-000000000046}'
    assert report['excel']['server_exists'] is True
    assert report['excel']['activation_attempted'] is False
    assert report['dependencies']['pywinauto'] == '0.6.9'
    assert report['platform'] == 'win32'


def test_cli_missing_exe_refuses_before_gui_import(tmp_path):
    result = subprocess.run([sys.executable, str(ROOT / 'harness.py'), '--exe', str(tmp_path / 'vba8.exe'), 'snapshot', '--pid', '0'], capture_output=True, text=True)
    assert result.returncode == 1
    assert 'Executable not built' in result.stderr
    assert 'pywinauto' not in result.stderr
    assert not (tmp_path / 'report').exists()


def test_window_scope_follows_owner_chain_not_arbitrary_window():
    from harness import in_scope
    pids = {10, 11}
    rows = {1: {'pid': 10, 'owner': 0}, 2: {'pid': 77, 'owner': 1}, 3: {'pid': 99, 'owner': 0}, 4: {'pid': 99, 'owner': 5}, 5: {'pid': 99, 'owner': 4}}
    assert in_scope(1, rows, pids)
    assert in_scope(2, rows, pids)
    assert not in_scope(3, rows, pids)
    assert not in_scope(4, rows, pids)


def test_action_target_requires_one_enabled_exact_match():
    from harness import choose_control
    import pytest
    class Info:
        def __init__(self, name, kind):
            self.name, self.control_type = name, kind
    class Control:
        def __init__(self, name, kind='Button', enabled=True):
            self.element_info, self.enabled = Info(name, kind), enabled
        def is_enabled(self):
            return self.enabled
    good = Control('Selecionar planilha')
    assert choose_control([good, Control('Gerar cópia')], 'Selecionar planilha', 'Button') is good
    with pytest.raises(ValueError, match='exactly one'):
        choose_control([good, good], 'Selecionar planilha', 'Button')
    with pytest.raises(ValueError, match='exactly one'):
        choose_control([good], 'Selecionar', 'Button')
    with pytest.raises(ValueError, match='disabled'):
        choose_control([Control('Gerar cópia', enabled=False)], 'Gerar cópia', 'Button')


def test_pid_must_match_exact_executable(tmp_path):
    from harness import validate_pid
    import os
    import pytest
    with pytest.raises(ValueError, match='does not match'):
        validate_pid(os.getpid(), tmp_path / 'vba8.exe')
    import psutil
    assert validate_pid(os.getpid(), Path(psutil.Process().exe())).pid == os.getpid()


def test_printwindow_uses_offscreen_bitmap_not_desktop_capture(tmp_path):
    # No app is launched: our own message-only STATIC window is not visible.
    from harness import print_window
    import win32gui
    handle = win32gui.CreateWindowEx(0, 'STATIC', 'VBA8 QA offscreen', 0, 0, 0, 180, 100, 0, 0, 0, None)
    try:
        result = print_window(handle, tmp_path / 'own-window.png')
        assert result['method'] == 'PrintWindow'
        assert result['desktop_fallback'] is False
        # A hidden window may refuse capture; this must be explicit, not a fake PNG.
        assert 'path' in result or 'error' in result
    finally:
        win32gui.DestroyWindow(handle)


def test_snapshot_tree_and_native_window_without_launch(tmp_path):
    from harness import enumerate_controls, scoped_windows
    import os
    import win32gui
    handle = win32gui.CreateWindowEx(0, 'STATIC', 'VBA8 QA owned test', 0, 0, 0, 180, 100, 0, 0, 0, None)
    try:
        windows = scoped_windows({os.getpid()}, visible_only=False)
        assert handle in windows
        from pywinauto import Desktop
        wrapper = Desktop(backend='win32').window(handle=handle).wrapper_object()
        rows = enumerate_controls(wrapper, max_nodes=10, max_depth=2)
        assert rows['controls'][0]['name'] == 'VBA8 QA owned test'
        assert rows['controls'][0]['handle'] == handle
        assert rows['truncated'] is False
    finally:
        win32gui.DestroyWindow(handle)


def test_report_on_own_hidden_window(tmp_path):
    from harness import capture_report
    import os
    import win32gui
    handle = win32gui.CreateWindowEx(0, 'STATIC', 'VBA8 QA report test', 0, 0, 0, 180, 100, 0, 0, 0, None)
    try:
        import psutil
        path = capture_report(os.getpid(), Path(psutil.Process().exe()), tmp_path / 'report', visible_only=False)
        data = json.loads(path.read_text(encoding='utf-8'))
        window = next(w for w in data['windows'] if w['handle'] == handle)
        assert window['win32']['controls'][0]['name'] == 'VBA8 QA report test'
        assert 'uia' in window
        assert window['screenshot']['desktop_fallback'] is False
        assert data['scope']['pid'] == os.getpid()
        assert data['status'] == 'inspection_only_not_e2e_pass'
    finally:
        win32gui.DestroyWindow(handle)
