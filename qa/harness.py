"""Opt-in native VBA8 QA; importing this module never starts an app."""
from pathlib import Path


def validate_exe(path):
    path = Path(path).resolve()
    if not path.is_file():
        raise FileNotFoundError(f'Executable not built: {path}')
    if path.name.lower() != 'vba8.exe':
        raise ValueError('Only vba8.exe may be targeted')
    return path


def preflight():
    """Registry inspection only. Do not CoCreate Excel or read Office settings."""
    import sys
    import winreg
    from importlib.metadata import version
    excel = {'activation_attempted': False}
    try:
        with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT, r'Excel.Application\CLSID') as key:
            excel['clsid'] = winreg.QueryValue(key, None)
        with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT, 'CLSID\\' + excel['clsid'] + r'\LocalServer32') as key:
            server = winreg.QueryValue(key, None)
        excel['server_command'] = server
        executable = server.split('"')[1] if server.startswith('"') else server.split(' /')[0]
        excel['server_exists'] = Path(executable.strip()).is_file()
    except OSError as exc:
        excel['error'] = str(exc)
    return {'platform': sys.platform, 'python': sys.version, 'excel': excel,
            'dependencies': {name: version(name) for name in ('pywinauto', 'pywin32', 'comtypes', 'pillow', 'psutil', 'pytest')}}


def validate_pid(pid, exe):
    import psutil
    process = psutil.Process(pid)
    if Path(process.exe()).resolve() != Path(exe).resolve():
        raise ValueError(f'PID {pid} does not match exact executable {exe}')
    return process


def in_scope(handle, rows, pids):
    seen = set()
    while handle in rows and handle not in seen:
        seen.add(handle)
        if rows[handle]['pid'] in pids:
            return True
        handle = rows[handle]['owner']
    return False


def scoped_windows(pids, visible_only=True):
    import win32gui
    import win32process
    rows = {}
    def collect(handle, _):
        rows[handle] = {'pid': win32process.GetWindowThreadProcessId(handle)[1],
                        'owner': win32gui.GetWindow(handle, 4)}
    win32gui.EnumWindows(collect, None)
    return {hwnd: row for hwnd, row in rows.items()
            if in_scope(hwnd, rows, pids) and (not visible_only or win32gui.IsWindowVisible(hwnd))}


def choose_control(controls, name, kind):
    """Exact unique selector helper for future driver; no fuzzy clicks."""
    matches = [c for c in controls if c.element_info.name == name and c.element_info.control_type == kind]
    if len(matches) != 1:
        raise ValueError(f'Expected exactly one {kind} named {name!r}; got {len(matches)}')
    if not matches[0].is_enabled():
        raise ValueError(f'Control {name!r} is disabled')
    return matches[0]


def enumerate_controls(root, max_nodes=1500, max_depth=20):
    """Bounded UIA/Win32 traversal. Record provider failures, never synthesize nodes."""
    controls, errors = [], []
    queue = [(root, 0, None)]
    truncated = False
    while queue and len(controls) < max_nodes:
        wrapper, depth, parent = queue.pop(0)
        try:
            info = wrapper.element_info
            rect = wrapper.rectangle()
            index = len(controls)
            controls.append({'index': index, 'parent': parent, 'depth': depth,
                             'name': info.name, 'control_type': getattr(info, 'control_type', ''),
                             'class_name': info.class_name, 'automation_id': getattr(info, 'automation_id', ''),
                             'handle': int(info.handle or 0), 'pid': info.process_id,
                             'enabled': wrapper.is_enabled(), 'visible': wrapper.is_visible(),
                             'rectangle': [rect.left, rect.top, rect.right, rect.bottom]})
            children = wrapper.children()
            if depth >= max_depth:
                truncated = truncated or bool(children)
            else:
                queue.extend((child, depth + 1, index) for child in children)
        except Exception as exc:
            errors.append({'depth': depth, 'error': f'{type(exc).__name__}: {exc}'})
    return {'controls': controls, 'errors': errors, 'truncated': truncated or bool(queue)}


def print_window(handle, destination):
    """Capture only the selected HWND using PrintWindow; no ImageGrab fallback."""
    import ctypes
    import win32gui
    import win32ui
    from PIL import Image, ImageStat
    result = {'method': 'PrintWindow', 'desktop_fallback': False, 'flags': 2}
    source = target = bitmap = old = None
    hdc = None
    try:
        left, top, right, bottom = win32gui.GetWindowRect(handle)
        width, height = right - left, bottom - top
        if width <= 0 or height <= 0 or width * height > 40_000_000:
            raise ValueError(f'Unsafe window dimensions: {width}x{height}')
        hdc = win32gui.GetWindowDC(handle)
        source = win32ui.CreateDCFromHandle(hdc)
        target = source.CreateCompatibleDC()
        bitmap = win32ui.CreateBitmap()
        bitmap.CreateCompatibleBitmap(source, width, height)
        old = target.SelectObject(bitmap)
        fn = ctypes.windll.user32.PrintWindow
        fn.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint]
        fn.restype = ctypes.c_bool
        if not fn(handle, target.GetSafeHdc(), 2):
            raise OSError('PrintWindow returned false; no desktop fallback attempted')
        image = Image.frombuffer('RGB', (width, height), bitmap.GetBitmapBits(True), 'raw', 'BGRX', 0, 1)
        image.save(destination)
        result.update({'path': str(Path(destination).resolve()), 'size': [width, height],
                       'warning': 'PrintWindow can omit GPU/WebView pixels; inspect PNG visually, success is not rendering proof',
                       'uniform_image': max(ImageStat.Stat(image).stddev) < 0.1})
    except Exception as exc:
        result['error'] = f'{type(exc).__name__}: {exc}'
    finally:
        if target and old:
            target.SelectObject(old)
        if bitmap:
            win32gui.DeleteObject(bitmap.GetHandle())
        if target:
            target.DeleteDC()
        if source:
            source.DeleteDC()
        if hdc:
            win32gui.ReleaseDC(handle, hdc)
    return result


def capture_report(pid, exe, output, visible_only=True):
    import json
    import datetime
    import win32gui
    from pywinauto import Desktop
    process = validate_pid(pid, exe)
    pids = {pid, *(child.pid for child in process.children(recursive=True))}
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'created': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'status': 'inspection_only_not_e2e_pass',
              'scope': {'pid': pid, 'exe': str(Path(exe).resolve()), 'process_tree': sorted(pids),
                        'owned_dialogs': True, 'foreground_requested': False}, 'windows': []}
    for handle, identity in scoped_windows(pids, visible_only).items():
        row = {'handle': handle, **identity, 'title': win32gui.GetWindowText(handle),
               'class_name': win32gui.GetClassName(handle)}
        row['screenshot'] = print_window(handle, output / f'window-{handle}.png')
        for backend in ('uia', 'win32'):
            try:
                wrapper = Desktop(backend=backend).window(handle=handle).wrapper_object()
                row[backend] = enumerate_controls(wrapper)
            except Exception as exc:
                row[backend] = {'error': f'{type(exc).__name__}: {exc}'}
        report['windows'].append(row)
    destination = output / 'report.json'
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
    return destination


def main(argv=None):
    import argparse
    import json
    import sys
    parser = argparse.ArgumentParser(description='Opt-in, read-only VBA8 native QA; never launches applications.')
    parser.add_argument('--exe', type=Path, default=Path(__file__).resolve().parents[1] / 'desktop/target/release/vba8.exe')
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('preflight', help='Detect dependencies/Excel registration without activating COM')
    snapshot = commands.add_parser('snapshot', help='Inspect only an already-running exact VBA8 executable')
    snapshot.add_argument('--pid', type=int, required=True)
    snapshot.add_argument('--output', type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == 'preflight':
            print(json.dumps(preflight(), ensure_ascii=False, indent=2))
        else:
            exe = validate_exe(args.exe)  # guard BEFORE importing pywinauto/UIA
            validate_pid(args.pid, exe)
            import datetime
            output = args.output or Path(__file__).resolve().parent / 'artifacts' / datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')
            print(capture_report(args.pid, exe, output))
        return 0
    except Exception as exc:
        print(f'{type(exc).__name__}: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
