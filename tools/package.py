#!/usr/bin/env python3
"""Package the mod as a source-only `.hnmod`, the file Horizon's launcher installs and builds.

The archive holds `horizon-native.json` and `mixins/voicechat/` (its `.rs` and `.toml` files, the
engine crate's included), with fixed dates and modes, so the same sources always give the same bytes.
`--install` also copies it into the launcher's native mods folder.
"""
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tomllib
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
INSTALLED = Path.home() / '.local/share/horizon/horizon-native-mods'

OPUS_BINARIES = {
    'opus-windows-x86_64.dll': {
        'url': 'https://repo.msys2.org/mingw/mingw64/mingw-w64-x86_64-opus-1.6.1-2-any.pkg.tar.zst',
        'sha256': 'fae981c518ed1ff2c5264eb5a0d590ec1d2ebb222defb2253ca85b2154fd8c04',
        'kind': 'zst',
        'member': 'mingw64/bin/libopus-0.dll',
        'license': 'mingw64/share/licenses/opus/COPYING',
    },
    'opus-macos-x86_64.dylib': {
        'url': 'https://ghcr.io/v2/homebrew/core/opus/blobs/sha256:0849d017c155c55ea1fb01b6156ff5dc77f2a2d7fec84592a099c32f6db9eef1',
        'sha256': '0849d017c155c55ea1fb01b6156ff5dc77f2a2d7fec84592a099c32f6db9eef1',
        'kind': 'gz',
        'member': 'opus/1.6.1/lib/libopus.0.dylib',
        'license': 'opus/1.6.1/COPYING',
        'headers': {'Authorization': 'Bearer QQ=='},
    },
    'opus-macos-aarch64.dylib': {
        'url': 'https://ghcr.io/v2/homebrew/core/opus/blobs/sha256:9b159efdfe3b5a31e5f4db9bac8cf40054ac301c508215f7db13cfc6ca661a66',
        'sha256': '9b159efdfe3b5a31e5f4db9bac8cf40054ac301c508215f7db13cfc6ca661a66',
        'kind': 'gz',
        'member': 'opus/1.6.1/lib/libopus.0.dylib',
        'license': 'opus/1.6.1/COPYING',
        'headers': {'Authorization': 'Bearer QQ=='},
    },
    'opus-linux-x86_64.so': {
        'url': 'https://ghcr.io/v2/homebrew/core/opus/blobs/sha256:739c259ce69a8af064bb73412700615a4e76405344c186f056710201f6ffda7b',
        'sha256': '739c259ce69a8af064bb73412700615a4e76405344c186f056710201f6ffda7b',
        'kind': 'gz',
        'member': 'opus/1.6.1/lib/libopus.so.0.11.1',
        'license': 'opus/1.6.1/COPYING',
        'headers': {'Authorization': 'Bearer QQ=='},
    },
}


def fetch(url, headers):
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers or {})) as response:
        return response.read()


def open_archive(archive, kind):
    if kind == 'gz':
        return tarfile.open(fileobj=io.BytesIO(archive), mode='r:gz')
    if kind == 'zst':
        raw = subprocess.run(['zstd', '-dc'], input=archive, stdout=subprocess.PIPE, check=True).stdout
        return tarfile.open(fileobj=io.BytesIO(raw), mode='r:')
    raise SystemExit(f'unknown archive kind {kind}')


def ensure_opus(bundle):
    data = bundle / 'data'
    data.mkdir(parents=True, exist_ok=True)
    for name, spec in OPUS_BINARIES.items():
        out = data / name
        if out.exists():
            continue
        archive = fetch(spec['url'], spec.get('headers'))
        got = hashlib.sha256(archive).hexdigest()
        if got != spec['sha256']:
            raise SystemExit(f"{name}: archive sha256 {got} does not match pinned {spec['sha256']}")
        with open_archive(archive, spec['kind']) as tar:
            member = tar.extractfile(spec['member'])
            if member is None:
                raise SystemExit(f"{name}: {spec['member']} missing or not a regular file")
            out.write_bytes(member.read())
            license_path = data / 'opus-LICENSE'
            if not license_path.exists():
                lic = tar.extractfile(spec['license'])
                if lic is not None:
                    license_path.write_bytes(lic.read())


def package():
    compat = json.loads((ROOT / 'compatibility.json').read_text())
    bundle = ROOT / 'mixins' / 'voicechat'
    mixin = tomllib.loads((bundle / 'horizon-mixin.toml').read_text())
    mod = mixin['mod']
    if mixin.get('compatibility', {}).get('horizon_revision') != compat['horizon_revision']:
        raise SystemExit('horizon-mixin.toml and compatibility.json pin different Horizon revisions')
    if (bundle / 'engine' / 'Cargo.lock').exists():
        raise SystemExit('remove mixins/voicechat/engine/Cargo.lock: a mod crate builds on Horizon\'s lock')
    ensure_opus(bundle)
    # No `hooks`: the mod reads game fields beyond the native hook points, so it
    # is pinned to the Horizon commit it was built and tested on.
    manifest = dict(schema=1, id=mod['id'], version=mod['version'], game=compat['game'],
                    horizon=compat['horizon_revision'], mixins=compat['horizon_mixins_revision'])
    files = {'horizon-native.json': (json.dumps(manifest, indent=2, sort_keys=True) + '\n').encode()}
    data_dir = bundle / 'data'
    for path in sorted(bundle.rglob('*')):
        if not path.is_file():
            continue
        if path.suffix in ('.rs', '.toml') or data_dir in path.parents:
            if path.is_symlink():
                raise SystemExit(f'{path.relative_to(ROOT)}: symlinks are not allowed in the bundle')
            files[path.relative_to(ROOT).as_posix()] = path.read_bytes()
    out = ROOT / 'target' / f"{mod['id']}-{mod['version']}.hnmod"
    out.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    result = dict(file=str(out), files=len(files), size=out.stat().st_size,
                  sha256=hashlib.sha256(out.read_bytes()).hexdigest())
    if '--install' in sys.argv[1:]:
        INSTALLED.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(out, INSTALLED / f"{mod['id']}.hnmod")
        result['installed'] = str(INSTALLED / f"{mod['id']}.hnmod")
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    package()
