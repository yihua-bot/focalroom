"""仅启动随机命名的隔离容器演练；不连接/修改已有项目，保留测试卷供复核。"""
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
from uuid import uuid4

spec = importlib.util.spec_from_file_location("backup_restore", Path(__file__).with_name("backup_restore.py"))
ops = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ops)
ROOT = Path(__file__).resolve().parents[2]


def run(args, expected=0, cwd=None):
    result = subprocess.run(args, capture_output=True, cwd=cwd, timeout=240)
    if result.returncode != expected:
        print(result.stdout.decode(), result.stderr.decode(), flush=True)
        raise AssertionError(f"fixture command exit={result.returncode}, expected={expected}")
    return result.stdout.decode()


def quote(value):
    return "'" + str(value).replace("'", "''") + "'"


def fingerprint(stack):
    result = {}
    for table in stack.facts()["tables"]:
        text = stack.query(f'SELECT coalesce(jsonb_agg(j ORDER BY j::text),\'[]\') FROM (SELECT to_jsonb(t) j FROM public."{table}" t) x')
        result[table] = ops.hashlib.sha256(text.encode()).hexdigest()
    return result


def seed(stack, directory):
    migrations = ["0001_shared_authority.sql", "0002_audit_envelopes.sql", "0006_rate_limits.sql", "0008_account_services.sql"]
    sql = "BEGIN;\n"
    for name in migrations:
        text = (ROOT / "apps/api/migrations" / name).read_text()
        sql += text + "\nINSERT INTO fr_migrations(version,sha256) VALUES(" + quote(name[:4]) + "," + quote(ops.hashlib.sha256(text.encode()).hexdigest()) + ");\n"
    sql += "INSERT INTO fr_users(id,email,display_name,password_hash) VALUES('u','fixture@focalroom.invalid','生成备份演练','test-not-a-login');\n"
    sql += "INSERT INTO fr_workspaces VALUES('w','生成备份演练','u'); INSERT INTO fr_memberships VALUES('w','u','OWNER');\n"
    snapshot = json.dumps({"photos": [{"id": "p1"}, {"id": "p2"}], "sequences": [{"id": "s", "items": [{"photo_id": "p1"}]}]})
    sql += "INSERT INTO fr_projects(id,workspace_id,owner_id,title,state,revision,snapshot,promotion_key,manifest_digest,initial_revision) VALUES('p','w','u','生成备份演练','SHARED',0," + quote(snapshot) + ",'promotion','manifest',0);\n"
    sql += "INSERT INTO fr_versions VALUES('p',0," + quote(snapshot) + ",'u','fixture-command');\n"
    sql += "INSERT INTO fr_commands(project_id,command_id,actor_id,idempotency_key,payload_digest,reply,envelope) VALUES('p','fixture-command','u','fixture-key','digest','{}','{\"kind\":\"fixture\"}');\n"
    sql += "INSERT INTO fr_comments(id,project_id,author_id,body,anchor) VALUES('c','p','u','生成评论，不计真人评测','{\"type\":\"project\"}');\n"
    for index in (1, 2):
        data = (ROOT.parent / "assets" / f"frame_{index:02}.jpg").read_bytes()
        digest = ops.hashlib.sha256(data).hexdigest()
        key = digest + ".jpg"
        local = directory / key; local.write_bytes(data)
        with local.open("rb") as source:
            ops.command(["docker", "exec", "-i", "--user", "0:0", stack.api[0], "sh", "-ec",
                         'umask 077; cat > "/data/previews/$1"; chown 1000:1000 "/data/previews/$1"', "fixture", key], stdin=source)
        sql += f"INSERT INTO fr_previews VALUES('p','p{index}',{quote(key)},{quote(digest)},64,64,{len(data)});\n"
    sql += "COMMIT;"
    script = directory / "seed.sql"; script.write_text(sql)
    with script.open("rb") as source:
        stack.pg_command('exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"', stdin=source)


def main():
    if os.environ.get("FOCALROOM_TEST_BACKUP_DOCKER") != "1":
        raise SystemExit("需显式设置 FOCALROOM_TEST_BACKUP_DOCKER=1；仅创建隔离演练栈")
    image = json.loads(run(["docker", "image", "inspect", "postgres:16-alpine"]))[0]["Id"]
    directory = Path(tempfile.mkdtemp(prefix="focalroom-backup-docker-"))
    # 不使用主工程 Compose、主机目录或端口；内部网络无对外出口，不拉取镜像。
    config = {"services": {
        "postgres": {"image": image, "pull_policy": "never", "environment": {"POSTGRES_USER": "focalroom", "POSTGRES_DB": "focalroom", "POSTGRES_HOST_AUTH_METHOD": "trust"}, "volumes": ["pg:/var/lib/postgresql/data"]},
        "api": {"image": image, "pull_policy": "never", "init": True, "entrypoint": ["sleep"], "command": ["infinity"], "user": "1000:1000", "environment": {"FOCALROOM_PREVIEW_DIRECTORY": "/data/previews", "FOCALROOM_DATABASE_URL": "postgresql://focalroom@postgres/focalroom"}, "volumes": ["previews:/data/previews"]}},
        "volumes": {"pg": {}, "previews": {}}, "networks": {"default": {"internal": True}}}
    compose = directory / "compose.json"; compose.write_text(json.dumps(config))
    suffix = uuid4().hex[:10]
    names = [f"fr-backup-{role}-{suffix}" for role in ("source", "target", "failed")]
    created, stacks, cases = [], [], []
    print(json.dumps({"fixture_directory": str(directory), "projects": names, "image": image}), flush=True)
    try:
        for name in names:
            args = ["docker", "compose", "-f", str(compose), "-p", name]
            created.append(args)
            run(args + ["up", "-d", "--scale", "api=2"])
            ready = False
            for _ in range(100):
                state = subprocess.run(args + ["exec", "-T", "postgres", "pg_isready", "-U", "focalroom", "-d", "focalroom"], capture_output=True)
                if state.returncode == 0:
                    ready = True; break
                time.sleep(0.1)
            assert ready
            stacks.append(ops.Stack(compose, name))
        source, target, failed = stacks
        seed(source, directory)
        original = fingerprint(source)
        # 一副本原本就停止，备份不得将它擅自启动。
        run(["docker", "stop", "--time", "1", source.api[1]])
        expected_states = [source.inspect(cid, "api")["running"] for cid in source.api]
        output = directory / "backups with spaces"
        text = run(["bash", str(ROOT / "ops/shared/backup.sh"), str(output), names[0], "--compose", str(compose)], cwd=directory)
        archive = Path(json.loads(text.splitlines()[-1])["backup"])
        assert archive.is_file() and archive.stat().st_mode & 0o777 == 0o600
        assert [source.inspect(cid, "api")["running"] for cid in source.api] == expected_states
        cases.append("backup-from-other-cwd-and-space-path-preserves-api-state")
        run(["bash", str(ROOT / "ops/shared/restore.sh"), str(archive)], cwd=directory)
        assert target.query("SELECT count(*) FROM pg_tables WHERE schemaname='public'") == "0"
        cases.append("verify-only-does-not-write-target")
        apply = ["bash", str(ROOT / "ops/shared/restore.sh"), str(archive), names[1], "--apply", "--confirm-project", names[1], "--compose", str(compose)]
        run(apply, cwd=directory)
        assert fingerprint(target) == original and fingerprint(source) == original
        assert not any(target.inspect(cid, "api")["running"] for cid in target.api)
        # 恢复保留1000:1000所有权；真实非root进程可读预览，不能仅用root校验。
        key = target.facts()["previews"][0]["object_key"]
        readable = target.helper(["stat", "-c", "%u:%g", "/previews/" + key]).decode().strip()
        assert readable == "1000:1000"
        nonroot = ops.command(["docker", "run", "--rm", "--pull=never", "--network", "none", "--read-only", "--user", "1000:1000",
                               "--mount", target.mount_arg + ",readonly", "--entrypoint", "sha256sum", image, "/previews/" + key])
        assert nonroot.decode().split()[0] + ".jpg" == key
        cases.append("pg-custom-dump-full-table-fingerprints-and-preview-ownership-roundtrip")
        run(apply, expected=1, cwd=directory)
        assert fingerprint(target) == original
        cases.append("nonempty-target-rejected-without-changing-data")
        # 有效校验和但不匹配的逻辑清单，在恢复后审计中必须失败并保持 API 停止。
        mismatch = directory / "mismatched-counts.tar.gz"
        with tarfile.open(archive, "r:gz") as src, tarfile.open(mismatch, "w:gz") as dest:
            for member in src:
                data = src.extractfile(member).read()
                if member.name == "manifest.json":
                    manifest = json.loads(data); manifest["database"]["tables"]["fr_comments"] += 1
                    data = json.dumps(manifest).encode(); member.size = len(data)
                dest.addfile(member, io.BytesIO(data))
        run(["bash", str(ROOT / "ops/shared/restore.sh"), str(mismatch), names[2], "--apply", "--confirm-project", names[2], "--compose", str(compose)], expected=1, cwd=directory)
        assert not any(failed.inspect(cid, "api")["running"] for cid in failed.api)
        cases.append("failed-post-restore-audit-keeps-target-api-stopped")
        source.query("UPDATE fr_previews SET object_key='missing.jpg' WHERE photo_id='p1'")
        run(["bash", str(ROOT / "ops/shared/backup.sh"), str(directory / "failed-backup"), names[0], "--compose", str(compose)], expected=1, cwd=directory)
        assert [source.inspect(cid, "api")["running"] for cid in source.api] == expected_states
        assert not list((directory / "failed-backup").glob("focalroom-backup-*.tar.gz"))
        cases.append("missing-preview-fails-backup-and-resumes-only-original-api")
        result = {"passed": True, "cases": cases, "postgres_image": image, "tables_verified": len(original), "preview_objects": 2,
                  "generated_fixture_only": True, "actual_api_processes": False, "real_docker_postgres": True,
                  "fixture_directory": str(directory), "projects": names, "volumes_retained": True}
        (directory / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2))
        print(json.dumps(result, ensure_ascii=False), flush=True)
    finally:
        for args in reversed(created):
            # 只停止本次新建项目；保留卷，不执行 down -v 或清理用户资源。
            subprocess.run(args + ["stop", "--timeout", "1"], capture_output=True, timeout=60)


if __name__ == "__main__":
    main()
