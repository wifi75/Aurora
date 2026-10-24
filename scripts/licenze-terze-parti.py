"""Rigenera docs/licenze-terze-parti.md: tutte le dipendenze di Aurora (Rust e interfaccia) con la loro licenza.

    python scripts/licenze-terze-parti.py

Serve avere installato `cargo` e le dipendenze dell'interfaccia (`cd app && npm install`).
Si rilancia quando cambiano le dipendenze; se compare una licenza non permissiva, lo script lo dice e finisce con errore.
"""

import collections
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# licenze compatibili con la distribuzione di Aurora sotto GPL-3.0-or-later
PERMISSIVE = ('MIT', 'Apache', 'BSD', 'ISC', 'Unicode', 'Zlib', '0BSD', 'CC0', 'MPL', 'Unlicense', 'BlueOak', 'OFL', 'CDLA-Permissive', 'BSL-1.0')
OWN = 'GPL-3.0-or-later'


def cargo(manifest):
    out = subprocess.run(['cargo', 'metadata', '--format-version', '1', '--manifest-path', manifest], capture_output=True, text=True, encoding='utf-8', cwd=ROOT, check=True).stdout
    data = json.loads(out)
    members = set(data['workspace_members'])
    return {(p['name'], p['version']): p.get('license') or p.get('license_file') or '??' for p in data['packages'] if p['id'] not in members}


def npm_production():
    pkg = json.load(open(os.path.join(ROOT, 'app', 'package.json'), encoding='utf-8'))
    found, seen = {}, set()

    def walk(name):
        path = os.path.join(ROOT, 'app', 'node_modules', *name.split('/'), 'package.json')
        if name in seen or not os.path.exists(path):
            return
        seen.add(name)
        j = json.load(open(path, encoding='utf-8'))
        lic = j.get('license') or '??'
        found[(name, j.get('version', '?'))] = lic if isinstance(lic, str) else lic.get('type', '??')
        for dep in j.get('dependencies') or {}:
            walk(dep)

    for dep in pkg.get('dependencies', {}):
        walk(dep)
    return found


rust = {}
rust.update(cargo(os.path.join(ROOT, 'core', 'Cargo.toml')))
rust.update(cargo(os.path.join(ROOT, 'desktop', 'src-tauri', 'Cargo.toml')))
npm = npm_production()

everything = {**{('rust ' + n, ver): lic for (n, ver), lic in rust.items()}, **{('npm ' + n, ver): lic for (n, ver), lic in npm.items()}}
problems = [(k, lic) for k, lic in everything.items() if lic != OWN and not any(p in lic for p in PERMISSIVE)]

lines = [
    '# Licenze delle dipendenze',
    '',
    'Elenco generato da `scripts/licenze-terze-parti.py` (non modificare a mano). Aurora e\' rilasciata con la GNU GPL 3 o successiva (`LICENSE`);',
    'le librerie qui sotto mantengono la propria licenza. Dove una libreria ne offre piu\' di una (`MIT OR Apache-2.0`) si usa la piu\' permissiva.',
    'Gli avvisi che accompagnano il programma sono in `THIRD-PARTY-NOTICES.md`.',
    '',
]
for title, items in (('Rust', {f'{n} {v}': lic for (n, v), lic in rust.items() if lic != OWN}), ('Interfaccia (solo cio\' che finisce nel programma)', {f'{n} {v}': lic for (n, v), lic in npm.items()})):
    counts = collections.Counter(items.values())
    lines += [f'## {title}: {len(items)} pacchetti', '', '| Licenza | Pacchetti |', '|---|---|']
    lines += [f'| {lic} | {n} |' for lic, n in counts.most_common()]
    lines += ['']
    by_license = collections.defaultdict(list)
    for name, lic in items.items():
        by_license[lic].append(name)
    for lic in sorted(by_license, key=lambda l: (-len(by_license[l]), l)):
        lines += [f'<details><summary>{lic} ({len(by_license[lic])})</summary>', '', ', '.join(sorted(by_license[lic])), '', '</details>', '']
open(os.path.join(ROOT, 'docs', 'licenze-terze-parti.md'), 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
print(f'{len(rust)} crate Rust, {len(npm)} pacchetti dell\'interfaccia')
if problems:
    print('LICENZE DA CONTROLLARE:')
    for (name, ver), lic in problems:
        print(' ', name, ver, lic)
    sys.exit(1)
print('Tutte le licenze sono permissive o compatibili con la GPL-3.0-or-later.')
