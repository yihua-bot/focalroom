"""归档与失败边界测试；不连接 Docker，不访问用户库或照片。"""
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("backup_restore", Path(__file__).with_name("backup_restore.py"))
ops = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ops)


class BackupTests(unittest.TestCase):
    def setUp(self):
        # 保留隔离文件用于复核；不执行删除。
        self.root = Path(tempfile.mkdtemp(prefix="focalroom-backup-unit-"))

    def bundle(self, *, preview_entries=None, outer_entries=None, mutate=None):
        preview = self.root / "previews.tar.gz"
        data = b"generated preview fixture"
        key = ops.hashlib.sha256(data).hexdigest() + ".jpg"
        with tarfile.open(preview, "w:gz") as archive:
            for name, value, kind in preview_entries or [(key, data, tarfile.REGTYPE)]:
                member = tarfile.TarInfo(name); member.type = kind; member.mode = 0o600
                member.size = len(value) if member.isfile() else 0
                if member.issym() or member.islnk():
                    member.linkname = "/etc/passwd"
                archive.addfile(member, io.BytesIO(value) if member.isfile() else None)
        (self.root / "database.dump").write_bytes(b"PGDMP-test-only")
        manifest = {"format": "focalroom-local-backup-v1", "source_project": "source-fixture", "postgres_major": 16,
                    "database": {"tables": {"fr_migrations": 1}, "previews": []},
                    "objects": {key: {"sha256": key[:-4], "bytes": len(data)}},
                    "checksums": {name: ops.sha256(self.root / name) for name in ("database.dump", "previews.tar.gz")}}
        if mutate:
            mutate(manifest)
        (self.root / "manifest.json").write_text(json.dumps(manifest))
        output = self.root / "bundle.tar.gz"
        with tarfile.open(output, "w:gz") as archive:
            for name in sorted(ops.MEMBERS):
                archive.add(self.root / name, arcname=name)
            for member in outer_entries or []:
                archive.addfile(member, io.BytesIO(b"x") if member.isfile() else None)
        return output

    def verify(self, archive):
        return ops.verify_bundle(archive, Path(tempfile.mkdtemp(dir=self.root)))

    def test_valid_archive_works_without_docker(self):
        with patch.object(ops, "Stack", side_effect=AssertionError("不得连接 Docker")):
            archive = self.bundle()
            self.assertEqual(len(self.verify(archive)["objects"]), 1)
            ops.restore(SimpleNamespace(archive=archive, apply=False))

    def test_outer_traversal_links_devices_and_duplicates_are_rejected(self):
        for name, kind in [("../outside", tarfile.REGTYPE), ("database.dump", tarfile.REGTYPE),
                           ("extra", tarfile.REGTYPE), ("manifest.json", tarfile.SYMTYPE),
                           ("device", tarfile.CHRTYPE)]:
            with self.subTest(name=name, kind=kind):
                member = tarfile.TarInfo(name); member.type = kind; member.size = 1 if member.isfile() else 0
                member.linkname = "/etc/passwd"
                with self.assertRaises(ops.BackupError):
                    self.verify(self.bundle(outer_entries=[member]))

    def test_preview_paths_links_hash_mismatch_and_duplicate_are_rejected(self):
        data = b"preview"
        key = ops.hashlib.sha256(data).hexdigest() + ".jpg"
        for entries in [[("../" + key, data, tarfile.REGTYPE)], [(key, data, tarfile.SYMTYPE)],
                        [(key, data, tarfile.LNKTYPE)], [(key, b"tampered", tarfile.REGTYPE)],
                        [(key, data, tarfile.REGTYPE), (key, data, tarfile.REGTYPE)]]:
            with self.subTest(entries=entries), self.assertRaises(ops.BackupError):
                self.verify(self.bundle(preview_entries=entries))

    def test_checksum_corruption_and_missing_references_are_rejected(self):
        with self.assertRaisesRegex(ops.BackupError, "校验和"):
            self.verify(self.bundle(mutate=lambda m: m["checksums"].update({"database.dump": "0" * 64})))
        def reference(manifest):
            manifest["database"]["previews"] = [{"object_key": "missing", "content_sha256": "0" * 64, "bytes": 1}]
        with self.assertRaisesRegex(ops.BackupError, "缺失"):
            self.verify(self.bundle(mutate=reference))

    def test_restore_requires_exact_confirmation_and_separate_target_before_docker(self):
        archive = self.bundle()
        for project, confirmation in [(None, None), ("target", "other"), ("source-fixture", "source-fixture")]:
            with self.subTest(project=project), patch.object(ops, "Stack", side_effect=AssertionError("不得连接 Docker")), self.assertRaises(ops.BackupError):
                ops.restore(SimpleNamespace(archive=archive, apply=True, project=project, confirm_project=confirmation))

    def test_backup_resumes_original_api_even_when_stopping_fails(self):
        stack = unittest.mock.Mock()
        stack.stop_api.side_effect = ops.BackupError("停服部分失败")
        with patch.object(ops, "Stack", return_value=stack), self.assertRaises(ops.BackupError):
            ops.backup(SimpleNamespace(output=self.root, project="fixture", compose="unused"))
        stack.resume_api.assert_called_once()
        self.assertEqual(list(self.root.glob("focalroom-backup-*.tar.gz")), [])

    def test_external_failure_body_is_not_logged(self):
        completed = SimpleNamespace(returncode=1, stdout=b"private config", stderr=b"secret database password")
        with patch.object(ops.subprocess, "run", return_value=completed), self.assertRaises(ops.BackupError) as error:
            ops.command(["docker", "config"])
        self.assertNotIn("secret", str(error.exception))

    def test_malformed_manifest_and_oversized_archive_fail_before_docker(self):
        archive = self.bundle()
        with patch.object(ops, "MAX_BYTES", 1), self.assertRaises(ops.BackupError):
            self.verify(archive)
        (self.root / "manifest.json").write_text("[]")
        with tarfile.open(archive, "w:gz") as output:
            for name in sorted(ops.MEMBERS):
                output.add(self.root / name, arcname=name)
        with self.assertRaises(ops.BackupError):
            self.verify(archive)

    def test_same_project_maintenance_excludes_concurrent_operation_and_releases(self):
        project = "fixture-" + self.root.name
        with ops.maintenance_lock(project):
            with self.assertRaisesRegex(ops.BackupError, "并发"):
                with ops.maintenance_lock(project):
                    self.fail("不应进入第二个运维操作")
            with ops.maintenance_lock(project + "-other"):
                pass
        with ops.maintenance_lock(project):
            pass


if __name__ == "__main__":
    unittest.main(verbosity=2)
