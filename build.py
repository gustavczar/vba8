"""Build local reproduzível. Requer Rust portátil e dependências npm instaladas."""
from pathlib import Path
import os
import subprocess
import sys

import shutil

ROOT = Path(__file__).resolve().parent

def resolve_tools(env=None, which=None):
    env = os.environ if env is None else env
    which = shutil.which if which is None else which
    result = []
    for name, variable in [('cargo', 'VBA8_CARGO'), ('npm', 'VBA8_NPM')]:
        value = env.get(variable) or which(name)
        if not value:
            raise RuntimeError(f'{name} não encontrado: instale a ferramenta no PATH ou defina {variable}.')
        result.append(str(value))
    return tuple(result)

def main():
    try:
        cargo, npm = resolve_tools()
        if not (ROOT / 'ui/node_modules').exists():
            raise RuntimeError('Execute npm ci --prefix ui antes do build.')
        commands = [
            ([npm, 'test'], ROOT / 'ui'),
            ([npm, 'run', 'build'], ROOT / 'ui'),
            ([cargo, 'test', '--locked'], ROOT / 'core'),
            ([cargo, 'test', '--locked'], ROOT / 'desktop'),
            ([cargo, 'build', '--release', '--locked'], ROOT / 'desktop'),
        ]
        for command, cwd in commands:
            print('RUN', subprocess.list2cmdline(command), 'IN', cwd, flush=True)
            subprocess.run(command, cwd=cwd, check=True)
        artifact = ROOT / 'desktop/target/release' / ('vba8.exe' if os.name == 'nt' else 'vba8')
        print('BUILD:', artifact, 'BYTES:', artifact.stat().st_size)
        return 0
    except (RuntimeError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1

if __name__ == '__main__':
    raise SystemExit(main())
