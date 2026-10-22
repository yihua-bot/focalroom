"""真实桥接可执行文件的无网络测试；只使用生成像素，不需要用户照片。"""
import argparse
import binascii
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import zlib

ROOT = Path(__file__).resolve().parent
BINARY = ROOT / 'build/FocalRoomBridge'


def png(path, color):
    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', binascii.crc32(tag + data) & 0xffffffff)
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 32, 24, 8, 2, 0, 0, 0))
                     + chunk(b'IDAT', zlib.compress((b'\0' + bytes(color) * 32) * 24)) + chunk(b'IEND', b''))


class BridgeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='focalroom-bridge-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "原片 ' $().png"
        self.render = self.root / 'render.png'
        png(self.source, [10, 20, 30]); png(self.render, [20, 30, 40])

    def request(self, provider='LIGHTROOM_CLASSIC', source=None):
        request = {'provider': provider, 'catalogId': 'private/测试.lrcat', 'destination': str(self.root),
                   'items': [{'sourcePath': str(source or self.source), 'renderPath': str(self.render),
                              'externalPhotoId': '42', 'externalVariantId': '42'}]}
        path = self.root / 'request.json'; result = self.root / 'result.json'
        path.write_text(json.dumps(request))
        process = subprocess.run([str(BINARY), '--request', str(path), str(result)], capture_output=True)
        return process, json.loads(result.read_text())

    def test_real_bytes_metadata_source_identity_and_repeated_export(self):
        original = self.source.read_bytes(); rendered = self.render.read_bytes()
        for provider in ['LIGHTROOM_CLASSIC', 'CAPTURE_ONE']:
            for _ in range(2):
                process, result = self.request(provider)
                self.assertEqual(process.returncode, 0, result)
                folder = Path(result['directory'])
                data = (folder / '.focalroom-manifest.json').read_text()
                manifest = json.loads(data); item = manifest['items'][0]
                self.assertEqual((folder / item['renderFilename']).read_bytes(), rendered)
                self.assertEqual(item['sourceSha256'], hashlib.sha256(original).hexdigest())
                self.assertEqual(item['renderSha256'], hashlib.sha256(rendered).hexdigest())
                self.assertEqual([item['width'], item['height'], item['bitDepth'], item['mediaFormat']], [32, 24, 8, 'PNG'])
                self.assertNotIn(str(self.root), data)
                self.assertNotIn('private/', data)
        self.assertEqual(self.source.read_bytes(), original)
        self.assertEqual(len(list(self.root.glob('FocalRoom-*'))), 4)

    def test_unsupported_provider_or_missing_source_publishes_no_batch(self):
        for provider, source in [('OTHER', self.source), ('CAPTURE_ONE', self.root / 'missing.raw')]:
            process, result = self.request(provider, source)
            self.assertNotEqual(process.returncode, 0); self.assertTrue(result['error'])
            self.assertEqual(list(self.root.glob('FocalRoom-*')), [])
            self.assertEqual(list(self.root.glob('.focalroom-partial-*')), [])

    def test_raw_or_corrupt_render_is_rejected(self):
        self.render.write_text('this is not an image')
        process, result = self.request()
        self.assertNotEqual(process.returncode, 0); self.assertTrue(result['error'])
        self.assertEqual(list(self.root.glob('FocalRoom-*')), [])


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixtures', action='store_true')
    args, rest = parser.parse_known_args()
    if args.fixtures:
        folder = Path(tempfile.mkdtemp(prefix='focalroom-bridge-native-'))
        png(folder / 'original.png', [10, 20, 30]); png(folder / 'edited.png', [20, 30, 40])
        print(folder)
    else:
        unittest.main(argv=['test_bridge.py', *rest])
