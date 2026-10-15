"""只读验证本轮独立原生测试库和生成素材；不执行 GUI 或业务写入。"""
import hashlib
import json
import pathlib
import sqlite3
import sys

EVIDENCE = pathlib.Path(__file__).resolve().parent
PROJECT = "31c253b6-6e1e-4041-b17f-2f58cec0e8d8"
DATABASE = pathlib.Path.home() / (
    "Library/Application Support/studio.betweenframes.m1reviewfixes/"
    "BetweenFrames/projects.sqlite"
)


def sha(value):
    return hashlib.sha256(value).hexdigest()


def verify():
    session = json.loads((EVIDENCE / "session.json").read_text())
    root = pathlib.Path(session["root"])
    manifest = json.loads((EVIDENCE / "source-manifest.json").read_text())
    assert len(manifest) == 500
    for name, expected in manifest.items():
        assert sha((root / "fixtures" / name).read_bytes()) == expected["sha256"], name
    connection = sqlite3.connect(DATABASE.as_uri() + "?mode=ro", uri=True)
    connection.execute("BEGIN")
    photos = connection.execute(
        "SELECT filename,content_sha256 FROM photos WHERE project_id=?", (PROJECT,)
    ).fetchall()
    assert len(photos) == 500
    assert all(manifest[name]["sha256"] == digest for name, digest in photos)
    decisions = connection.execute(
        "SELECT status,rating,count(*) FROM photo_decisions "
        "WHERE project_id=? GROUP BY status,rating", (PROJECT,)
    ).fetchall()
    sequence = connection.execute(
        "SELECT p.filename,s.position FROM sequence_items s JOIN photos p ON p.id=s.photo_id "
        "WHERE s.project_id=? ORDER BY s.position", (PROJECT,)
    ).fetchall()
    assert len(sequence) == len({position for _, position in sequence}) == 25
    assert connection.execute("SELECT count(*) FROM agent_tasks").fetchone()[0] == 0
    tables = {}
    for (table,) in connection.execute("SELECT name FROM sqlite_master WHERE type='table'"):
        quoted = '"' + table.replace('"', '""') + '"'
        rows = connection.execute("SELECT * FROM " + quoted).fetchall()
        serialized = sorted(json.dumps(row, ensure_ascii=False, default=lambda b: b.hex()) for row in rows)
        tables[table] = {"rows": len(rows), "sha256": sha("\n".join(serialized).encode())}
    connection.close()
    return {"projectId": PROJECT, "sourceHashesUnchanged": 500, "decisions": decisions,
            "sequence": sequence, "agentTasks": 0, "tables": tables}


if __name__ == "__main__":
    # 固定标签避免任意目标路径；复现前先确保 session 中的临时素材仍存在。
    label = sys.argv[1]
    assert label in ("before-rebuild", "after-rebuild", "before-restart", "after-restart", "after-export")
    result = json.loads(json.dumps(verify()))
    if label in ("after-restart", "after-rebuild"):
        before = json.loads((EVIDENCE / label.replace("after-", "before-")).with_suffix(".json").read_text())
        assert result == before, "重启前后业务/审计/辅助表内容不同"
    (EVIDENCE / (label + ".json")).write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print(json.dumps({"stage": label, "tables": len(result["tables"]),
                      "sourceHashesUnchanged": 500, "decisions": result["decisions"],
                      "firstSequenceNames": result["sequence"][:3], "agentTasks": 0}, ensure_ascii=False))
