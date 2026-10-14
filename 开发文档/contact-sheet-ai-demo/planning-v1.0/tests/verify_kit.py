#!/usr/bin/env python3
"""Static contract + new-install SQLite migration verification.
Runs without external dependencies. This is not an app/E2E test.
"""
import json, sqlite3
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
migrations=sorted((ROOT/'migrations').glob('*.sql'))
assert len(migrations) >= 2
connection=sqlite3.connect(':memory:')
connection.execute('PRAGMA foreign_keys=ON')
for migration in migrations:
    connection.executescript(migration.read_text())
    print('MIGRATION PASS:', migration.name)
assert connection.execute('PRAGMA foreign_key_check').fetchall()==[]
counts=connection.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall()
assert len(counts)>=20, counts
schemas={}
for f in sorted((ROOT/'contracts').glob('*.json')):
    d=json.loads(f.read_text()); schemas[f.name]=d
    print('CONTRACT JSON PASS:', f.name)
assert 'tools' in schemas['agent-tool-contracts.json']
assert len(schemas['agent-tool-contracts.json']['tools'])==12
matrix=json.loads((ROOT/'tests/AGENT-ACCEPTANCE-MATRIX.json').read_text())
assert len(matrix['cases'])==15
assert len(set(x['id'] for x in matrix['cases']))==15
print('ACCEPTANCE MATRIX PASS:', len(matrix['cases']))
# Test runtime state includes 'paused' when new baseline is applied.
connection.execute("INSERT INTO projects(id,title) VALUES ('p1','test')")
connection.execute("INSERT INTO agent_tasks(id,project_id,base_project_revision,goal,state) VALUES ('t1','p1',0,'test','paused')")
connection.execute("INSERT INTO privacy_consents(id,project_id,task_id,provider_id,scope,status,max_image_count) VALUES ('c1','p1','t1','mock','single_task','granted',10)")
connection.execute("INSERT INTO agent_checkpoints(id,task_id,ordinal,task_state,project_revision) VALUES ('cp1','t1',1,'paused',0)")
connection.execute("INSERT INTO domain_idempotency(idempotency_key,command_id,project_id,response_json,command_hash) VALUES ('ik','cid','p1','{}','abc')")
connection.commit()
assert connection.execute('PRAGMA foreign_key_check').fetchall()==[]
print('DB INTEGRITY PASS: ', len(counts),'tables, paused runtime, consent, checkpoint, idempotency')
print('PASS (static/migration checks only; no production runtime verified)')
