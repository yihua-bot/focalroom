"""M4 私有磁盘预览备份/空项目恢复。Python 3.11+ 标准库，不拉取新镜像。"""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import stat
import tarfile
import tempfile
from datetime import datetime, timezone
from uuid import uuid4
from urllib.parse import urlsplit

MAX_BYTES = 64 * 1024**3
MAX_FILES = 100_000
MEMBERS = {"manifest.json", "database.dump", "previews.tar.gz"}
OBJECT = re.compile(r"[a-f0-9]{64}\.jpg")


class BackupError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise BackupError(message)


def private_file(path):
    return os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), "wb")


@contextmanager
def maintenance_lock(project):
    # 同一主机/用户/项目的两次运维不能互相恢复对方停写的 API。
    key = hashlib.sha256(project.encode()).hexdigest()
    path = Path(tempfile.gettempdir()) / f"focalroom-maintenance-{os.getuid()}-{key}.lock"
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        info = os.fstat(fd)
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid(), "运维锁文件不可信")
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise BackupError("该项目已有备份或恢复操作；拒绝并发运维") from error
        yield
    finally:
        os.close(fd)  # 保留锁文件，避免删除造成 inode 竞争。


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def command(args, *, stdin=None, stdout=None):
    try:
        result = subprocess.run(args, stdin=stdin, stdout=stdout or subprocess.PIPE,
                                stderr=subprocess.PIPE, timeout=600, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise BackupError("命令不可用或超时；保留当前数据与诊断目录") from error
    # 不输出 Docker config、数据库正文、环境变量或第三方错误。
    require(result.returncode == 0, f"命令执行失败（exit={result.returncode}）；未跳过错误")
    return result.stdout


def preview_inventory(path):
    inventory, total = {}, 0
    with tarfile.open(path, "r:gz") as archive:
        for member in archive:
            name = member.name.removeprefix("./")
            if name in ("", ".") and member.isdir():
                continue
            require(member.isfile() and bool(OBJECT.fullmatch(name)), "预览归档包含非法路径或非普通文件")
            require(name not in inventory and len(inventory) < MAX_FILES, "预览归档有重复文件或文件过多")
            require(0 < member.size <= 8 * 1024**2 and not member.mode & 0o7111, "预览大小或权限不合法")
            total += member.size
            require(total <= MAX_BYTES, "预览归档展开后过大")
            with archive.extractfile(member) as stream:
                actual = hashlib.file_digest(stream, "sha256").hexdigest()
            require(actual + ".jpg" == name, "预览内容与对象名 SHA256 不一致")
            inventory[name] = {"sha256": actual, "bytes": member.size}
    return inventory


def check_references(facts, inventory):
    require(isinstance(facts, dict) and isinstance(facts.get("tables"), dict)
            and "fr_migrations" in facts["tables"] and isinstance(facts.get("previews"), list), "缺少应用数据库清单")
    for row in facts["previews"]:
        require(isinstance(row, dict) and isinstance(row.get("object_key"), str), "预览引用结构不合法")
        key = row["object_key"]
        require(key in inventory and inventory[key]["sha256"] == row["content_sha256"]
                and inventory[key]["bytes"] == row["bytes"], "数据库引用的预览缺失或内容不一致")


def verify_bundle(path, directory):
    require(path.is_file() and path.stat().st_size <= MAX_BYTES, "备份文件不可用或过大")
    found, total = set(), 0
    with tarfile.open(path, "r:gz") as archive:
        for member in archive:
            require(member.name in MEMBERS and member.name not in found and member.isfile(), "备份只能包含三个不重复的普通文件")
            total += member.size
            require(0 < member.size <= MAX_BYTES and total <= MAX_BYTES, "备份展开后过大")
            if member.name == "manifest.json":
                require(member.size <= 32 * 1024**2, "备份清单过大")
            # 不用 extract/extractall，不接受软硬链接、设备或路径穿越。
            with archive.extractfile(member) as source, private_file(directory / member.name) as output:
                while chunk := source.read(1024**2):
                    output.write(chunk)
            found.add(member.name)
    require(found == MEMBERS, "备份成员不完整")
    manifest = json.loads((directory / "manifest.json").read_text())
    require(isinstance(manifest, dict) and manifest.get("format") == "focalroom-local-backup-v1", "备份格式不支持；旧版归档须独立迁移")
    require(isinstance(manifest.get("checksums"), dict), "缺少校验清单")
    require(set(manifest.get("checksums", {})) == {"database.dump", "previews.tar.gz"}, "校验清单不完整")
    for name, expected in manifest["checksums"].items():
        require(re.fullmatch(r"[a-f0-9]{64}", expected) and sha256(directory / name) == expected, "备份校验和不一致")
    with (directory / "database.dump").open("rb") as stream:
        require(stream.read(5) == b"PGDMP", "数据库不是 PostgreSQL custom dump")
    inventory = preview_inventory(directory / "previews.tar.gz")
    require(inventory == manifest.get("objects"), "预览对象清单不一致")
    check_references(manifest["database"], inventory)
    return manifest


class Stack:
    def __init__(self, compose, project):
        require(bool(re.fullmatch(r"[a-z0-9][a-z0-9_-]{0,62}", project)), "项目名不合法")
        self.project = project
        self.compose = ["docker", "compose", "-f", str(compose), "-p", project]
        config = json.loads(command(self.compose + ["config", "--format", "json"]))
        services = config["services"]
        require(set(services).issubset({"postgres", "api", "web"}) and {"postgres", "api"} <= set(services), "仅支持独立 postgres/api/web 栈；其他写入者须单独停写方案")
        env = services["api"].get("environment", {})
        require(not env.get("FOCALROOM_R2_CONFIG") and env.get("FOCALROOM_PREVIEW_DIRECTORY") == "/data/previews", "本工具仅支持磁盘预览；R2 须独立备份方案")
        database = urlsplit(env.get("FOCALROOM_DATABASE_URL", ""))
        expected_db = services["postgres"].get("environment", {}).get("POSTGRES_DB")
        require(database.scheme in ("postgres", "postgresql") and database.hostname == "postgres"
                and database.port in (None, 5432) and database.path == "/" + str(expected_db), "API 必须绑定本项目 postgres 服务的同一数据库")
        self.pg, self.api = self.ids("postgres"), self.ids("api")
        require(len(self.pg) == 1 and self.api, "必须先创建唯一 PostgreSQL 和至少一个 API 容器")
        self.pg_info = self.inspect(self.pg[0], "postgres")
        require(self.pg_info["running"], "PostgreSQL 未运行")
        require(self.query("SELECT current_database()") == expected_db, "容器实际数据库与 Compose 配置不一致")
        mounts = [[m for m in self.inspect(cid, "api")["mounts"] if m["Destination"] == "/data/previews"] for cid in self.api]
        require(all(len(items) == 1 for items in mounts), "API 预览目录必须是明确挂载")
        self.mount = mounts[0][0]
        require(self.mount["Type"] in ("volume", "bind") and all(items[0]["Source"] == self.mount["Source"] for items in mounts), "多个 API 必须使用同一预览存储")
        source = self.mount.get("Name") if self.mount["Type"] == "volume" else self.mount["Source"]
        require(source and not any(c in source for c in ",\n\x00"), "挂载来源不支持")
        self.mount_arg = f"type={self.mount['Type']},source={source},target=/previews"
        self.image = self.pg_info["image"]

    def ids(self, service):
        return command(self.compose + ["ps", "--all", "--quiet", service]).decode().split()

    def inspect(self, cid, service):
        template = '{"mounts":{{json .Mounts}},"labels":{{json .Config.Labels}},"running":{{json .State.Running}},"image":{{json .Image}}}'
        info = json.loads(command(["docker", "inspect", "--format", template, cid]))
        require(info["labels"].get("com.docker.compose.project") == self.project
                and info["labels"].get("com.docker.compose.service") == service, "容器项目/服务身份不匹配")
        return info

    def pg_command(self, script, *, stdin=None, stdout=None):
        return command(self.compose + ["exec", "-T", "postgres", "sh", "-ec", script], stdin=stdin, stdout=stdout)

    def query(self, sql):
        # SQL 为内部常量；数据库身份从容器环境读取，不在参数中传密码。
        return self.pg_command('exec psql -X -v ON_ERROR_STOP=1 -At -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c ' + shlex.quote(sql)).decode().strip()

    def facts(self):
        tables = json.loads(self.query("SELECT coalesce(json_agg(tablename ORDER BY tablename),'[]') FROM pg_tables WHERE schemaname='public'"))
        require(tables and all(re.fullmatch(r"fr_[a-z0-9_]+", name) for name in tables), "数据库不是独立 FocalRoom 应用库")
        return {
            "tables": {name: int(self.query(f'SELECT count(*) FROM public."{name}"')) for name in tables},
            "migrations": json.loads(self.query("SELECT coalesce(json_agg(t ORDER BY version),'[]') FROM (SELECT version,sha256 FROM fr_migrations) t")),
            "previews": json.loads(self.query("SELECT coalesce(json_agg(t ORDER BY project_id,photo_id),'[]') FROM (SELECT project_id,photo_id,object_key,content_sha256,bytes FROM fr_previews) t")),
        }

    def helper(self, args, *, write=False, stdin=None, stdout=None):
        mount = self.mount_arg + ("" if write else ",readonly")
        return command(["docker", "run"] + (["-i"] if stdin is not None else []) + ["--rm", "--pull=never", "--network", "none", "--read-only", "--user", "0:0",
                        "--mount", mount, "--entrypoint", args[0], self.image] + args[1:], stdin=stdin, stdout=stdout)

    def stop_api(self):
        running = [cid for cid in self.api if self.inspect(cid, "api")["running"]]
        self.previously_running = running  # 部分停服失败也保留原运行集合。
        if running:
            command(["docker", "stop", "--time", "30"] + running)
        require(not any(self.inspect(cid, "api")["running"] for cid in self.api), "API 停写失败")

    def resume_api(self):
        for cid in getattr(self, "previously_running", []):
            if not self.inspect(cid, "api")["running"]:
                command(["docker", "start", cid])
            require(self.inspect(cid, "api")["running"], "API 未恢复运行；请人工检查")

    def require_empty(self):
        count = int(self.query("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema'"))
        require(count == 0, "拒绝恢复：目标数据库不是空库")
        entries = self.helper(["find", "/previews", "-mindepth", "1", "-print", "-quit"])
        require(not entries.strip(), "拒绝恢复：目标预览目录不是空目录")


def backup(args):
    with maintenance_lock(args.project):
        return create_backup(args)


def create_backup(args):
    stack = Stack(args.compose, args.project)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True, mode=0o700)
    stage = Path(tempfile.mkdtemp(prefix=".focalroom-backup-", dir=output))
    print(f"备份暂存目录：{stage}", flush=True)
    try:
        print("暂停本项目 API，建立数据库与磁盘预览一致快照", flush=True)
        stack.stop_api()
        facts = stack.facts()
        version = int(stack.query("SHOW server_version_num")) // 10000
        with private_file(stage / "database.dump") as target:
            stack.pg_command('exec pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --format=custom --no-owner --no-privileges', stdout=target)
        with private_file(stage / "previews.tar.gz") as target:
            stack.helper(["tar", "-czf", "-", "-C", "/previews", "."], stdout=target)
        inventory = preview_inventory(stage / "previews.tar.gz")
        check_references(facts, inventory)
        require(stack.facts() == facts, "备份过程中数据库变化；请检查外部写入者")
        manifest = {"format": "focalroom-local-backup-v1", "created_at": datetime.now(timezone.utc).isoformat(),
                    "source_project": args.project, "postgres_major": version, "database": facts,
                    "objects": inventory, "checksums": {name: sha256(stage / name) for name in ("database.dump", "previews.tar.gz")}}
        manifest_bytes = (json.dumps(manifest, ensure_ascii=False, indent=2) + "\n").encode()
        require(len(manifest_bytes) <= 32 * 1024**2, "备份清单过大；未发布归档")
        with private_file(stage / "manifest.json") as target:
            target.write(manifest_bytes)
    finally:
        stack.resume_api()  # 不用 compose up，不启动原先停止的服务。
    name = "focalroom-backup-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid4().hex[:8] + ".tar.gz"
    archive = output / name
    partial = archive.with_suffix(archive.suffix + ".partial")
    require(sum((stage / name).stat().st_size for name in MEMBERS) <= MAX_BYTES, "备份超过支持大小；未发布归档")
    with private_file(partial) as target, tarfile.open(fileobj=target, mode="w:gz") as tar:
        for member in sorted(MEMBERS):
            tar.add(stage / member, arcname=member, recursive=False)
    os.rename(partial, archive)
    print(json.dumps({"backup": str(archive), "objects": len(inventory), "postgres_major": version, "api_resumed": True}), flush=True)


def restore(args):
    stage = Path(tempfile.mkdtemp(prefix="focalroom-restore-"))
    print(f"验证暂存目录：{stage}", flush=True)
    manifest = verify_bundle(args.archive.resolve(), stage)
    print("归档 SHA256、成员路径、数据库格式头与全部预览引用验证通过", flush=True)
    if not args.apply:
        print(json.dumps({"verified": True, "applied": False, "source_project": manifest["source_project"], "objects": len(manifest["objects"])}))
        return
    require(args.project and args.confirm_project == args.project, "执行恢复须显式指定项目并以 --confirm-project 重复确认")
    require(args.project != manifest["source_project"], "拒绝覆盖备份来源项目；请使用独立空目标")
    with maintenance_lock(args.project):
        apply_restore(args, manifest, stage)


def apply_restore(args, manifest, stage):
    stack = Stack(args.compose, args.project)
    require(int(stack.query("SHOW server_version_num")) // 10000 == manifest["postgres_major"], "源与目标 PostgreSQL 主版本不一致")
    stack.require_empty()
    with (stage / "database.dump").open("rb") as source:
        stack.pg_command("exec pg_restore --list", stdin=source)
    print("目标为空，暂停目标 API；失败后保持停止，不暴露半恢复状态", flush=True)
    stack.stop_api()
    stack.require_empty()
    print("恢复磁盘预览到空目录", flush=True)
    with (stage / "previews.tar.gz").open("rb") as source:
        stack.helper(["tar", "-xzf", "-", "-C", "/previews"], write=True, stdin=source)
    print("单事务恢复 PostgreSQL，不覆盖已有表", flush=True)
    with (stage / "database.dump").open("rb") as source:
        stack.pg_command('exec pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --single-transaction --exit-on-error --no-owner --no-privileges', stdin=source)
    require(stack.facts() == manifest["database"], "恢复后表计数/迁移/预览引用不一致；API 保持停止")
    with private_file(stage / "restored-previews.tar.gz") as target:
        stack.helper(["tar", "-czf", "-", "-C", "/previews", "."], stdout=target)
    require(preview_inventory(stage / "restored-previews.tar.gz") == manifest["objects"], "恢复后预览 SHA256 不一致；API 保持停止")
    print(json.dumps({"restored": True, "project": args.project, "objects": len(manifest["objects"]), "api_stopped": True,
                      "next": "先完成应用、权限与备份审阅，再显式启动目标 API/Web"}), flush=True)


def main():
    if sys.version_info < (3, 11):
        print("需要 Python 3.11 或更新版本", file=sys.stderr)
        return 1
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    b = sub.add_parser("backup")
    b.add_argument("output", type=Path)
    b.add_argument("project", nargs="?", default="focalroom-shared-review")
    r = sub.add_parser("restore")
    r.add_argument("archive", type=Path)
    r.add_argument("project", nargs="?")
    r.add_argument("--apply", action="store_true")
    r.add_argument("--confirm-project")
    for child in (b, r):
        child.add_argument("--compose", type=lambda value: Path(value).resolve(), default=Path(__file__).resolve().with_name("compose.yaml"))
    args = parser.parse_args()
    try:
        (backup if args.action == "backup" else restore)(args)
    except (BackupError, OSError, ValueError, KeyError, TypeError, tarfile.TarError) as error:
        message = str(error) if isinstance(error, BackupError) else "归档或配置无效；未绕过验证，保留暂存目录供审查"
        print("失败：" + message, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
