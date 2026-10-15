"""只读核对本轮独立评审库、CR2 副本与上一轮 500 图项目。"""
import hashlib
import json
import pathlib
import sqlite3
import sys

ROOT = pathlib.Path(__file__).resolve().parent
DATA = pathlib.Path.home() / "Library/Application Support/studio.betweenframes.m1reviewfixes/BetweenFrames"
stage = sys.argv[1]
assert stage in ("after-restart", "final")
fixture = json.loads((ROOT / "native-fixture.json").read_text())
assert hashlib.sha256(pathlib.Path(fixture["source"]).read_bytes()).hexdigest() == fixture["sha256"]
connection = sqlite3.connect((DATA / "projects.sqlite").as_uri() + "?mode=ro", uri=True)
connection.execute("BEGIN")
tables = {}
for (name,) in connection.execute("SELECT name FROM sqlite_master WHERE type='table'"):
    quoted = '"' + name.replace('"', '""') + '"'
    rows = connection.execute("SELECT * FROM " + quoted).fetchall()
    serialized = sorted(json.dumps(row, ensure_ascii=False, default=lambda value: value.hex()) for row in rows)
    tables[name] = {"rows": len(rows), "sha256": hashlib.sha256("\n".join(serialized).encode()).hexdigest()}
assert tables == json.loads((ROOT / "before-restart.json").read_text())
assert connection.execute("SELECT count(*) FROM agent_tasks").fetchone()[0] == 0
cache = {str(path.relative_to(DATA)): path.stat().st_size for path in DATA.rglob("*.jpg") if path.is_file()}
assert cache == json.loads((ROOT / "cache-before-inspection.json").read_text())

previous = ROOT.parent / "v2-m1-native-500"
old = json.loads((previous / "after-restart.json").read_text())
session = json.loads((previous / "session.json").read_text())
manifest = json.loads((previous / "source-manifest.json").read_text())
for name, expected in manifest.items():
    path = pathlib.Path(session["root"]) / "fixtures" / name
    assert hashlib.sha256(path.read_bytes()).hexdigest() == expected["sha256"]
sequence = connection.execute(
    "SELECT p.filename,s.position FROM sequence_items s JOIN photos p ON p.id=s.photo_id "
    "WHERE s.project_id=? ORDER BY s.position", (old["projectId"],)
).fetchall()
assert json.loads(json.dumps(sequence)) == old["sequence"]
decisions = connection.execute(
    "SELECT status,rating,count(*) FROM photo_decisions WHERE project_id=? GROUP BY status,rating",
    (old["projectId"],)
).fetchall()
assert json.loads(json.dumps(decisions)) == old["decisions"]
result = {"equalTables": len(tables), "cr2SourceHashUnchanged": True,
          "noNewJpegCache": True, "agentTasks": 0, "old500SourceHashesUnchanged": 500,
          "old500SequenceAndDecisionsUnchanged": True, "tables": tables}
(ROOT / (stage + ".json")).write_text(json.dumps(result, indent=2))
print(json.dumps({key: value for key, value in result.items() if key != "tables"}))
