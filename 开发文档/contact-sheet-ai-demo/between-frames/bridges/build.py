"""构建独立 macOS 插件包；只写入 bridges/build，不安装、不发布、不公证。"""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'build'
VERSION = '1.0.0'


def run(*args):
    subprocess.run([str(arg) for arg in args], check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--sign', help='Developer ID Application identity; omitted builds are local only')
    args = parser.parse_args()
    OUT.mkdir(exist_ok=True)
    source = ROOT / 'macos/Bridge.swift'
    binaries = []
    for arch in ['arm64', 'x86_64']:
        binary = OUT / f'bridge-{arch}'
        run('xcrun', 'swiftc', source, '-O', '-target', f'{arch}-apple-macos11.0', '-o', binary)
        binaries.append(binary)
    binary = OUT / 'FocalRoomBridge'
    run('lipo', '-create', *binaries, '-output', binary)
    app = OUT / 'FocalRoom Capture One Bridge.app'
    executable = app / 'Contents/MacOS/FocalRoomBridge'
    executable.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable)
    resources = app / 'Contents/Resources'
    resources.mkdir(exist_ok=True)
    shutil.copy2(ROOT.parent / 'src-tauri/icons/icon.icns', resources / 'AppIcon.icns')
    info = {
        'CFBundleName': 'FocalRoom Capture One Bridge', 'CFBundleDisplayName': 'FocalRoom Capture One Bridge',
        'CFBundleIdentifier': 'cc.focroom.captureone.bridge', 'CFBundleExecutable': 'FocalRoomBridge',
        'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': VERSION, 'CFBundleVersion': '1',
        'LSMinimumSystemVersion': '11.0', 'NSHighResolutionCapable': True, 'CFBundleIconFile': 'AppIcon',
        'CFBundleDocumentTypes': [{'CFBundleTypeName': 'Exported photographs', 'CFBundleTypeRole': 'Viewer',
            'LSHandlerRank': 'Alternate', 'LSItemContentTypes': ['public.jpeg', 'public.png', 'public.tiff']}],
    }
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    plugin = OUT / 'FocalRoom.lrplugin'
    shutil.copytree(ROOT / 'lightroom-classic/FocalRoom.lrplugin', plugin, dirs_exist_ok=True)
    (plugin / 'bin').mkdir(exist_ok=True)
    shutil.copy2(binary, plugin / 'bin/FocalRoomBridge')
    if args.sign:
        for code in [executable, plugin / 'bin/FocalRoomBridge', app]:
            run('codesign', '--force', '--timestamp', '--options', 'runtime', '--sign', args.sign, code)
        run('codesign', '--verify', '--deep', '--strict', '--verbose=2', app)
    files = []
    for name, content in [('FocalRoom-Lightroom-Classic', plugin), ('FocalRoom-Capture-One-Bridge', app)]:
        folder = OUT / f'{name}-{VERSION}'
        folder.mkdir(exist_ok=True)
        shutil.copytree(content, folder / content.name, dirs_exist_ok=True)
        shutil.copy2(ROOT / 'INSTALL.md', folder / 'INSTALL.md')
        archive = OUT / f'{name}-{VERSION}-macos.zip'
        run('ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', folder, archive)
        files.append({'file': archive.name, 'bytes': archive.stat().st_size,
                      'sha256': hashlib.sha256(archive.read_bytes()).hexdigest()})
    (OUT / 'checksums.json').write_text(json.dumps({'version': VERSION, 'signed': bool(args.sign), 'files': files}, indent=2) + '\n')
    print(json.dumps(files, indent=2))


if __name__ == '__main__':
    main()
