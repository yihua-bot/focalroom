"""Package quality checks. Run: python tests/validate_contracts.py"""
import json
import sqlite3
from pathlib import Path
ROOT=Path(__file__).parents[1]
files=list(ROOT.rglob('*.json'))
for f in files:
    json.loads(f.read_text(encoding='utf-8'))
print(f'OK: parsed {len(files)} JSON files')

tools=json.loads((ROOT/'contracts/agent-tool-contracts.json').read_text())['tools']
assert len(tools)==12, len(tools)
assert not any(t['name']=='ApplyDraft' or t['name']=='apply_draft' for t in tools)
for t in tools:
    p=t['parameters']
    assert t['strict'] is True
    assert p['type']=='object' and p['additionalProperties'] is False
    assert set(p['properties'])==set(p['required'])
print('OK: 12 strict tools, no privileged ApplyDraft')

cmd=json.loads((ROOT/'contracts/command-envelope.schema.json').read_text())
assert len(cmd['properties']['command']['oneOf'])==12
try:
    import jsonschema
    jsonschema.Draft202012Validator.check_schema(cmd)
    good={
      'commandId':'cmd-1','projectId':'p1','actorType':'human',
      'source':'gui_direct','expectedProjectRevision':0,'causationId':None,
      'command':{'type':'SetPhotoDecision','photoIds':['a'],'status':'keep'}
    }
    jsonschema.validate(good,cmd)
    print('OK: command envelope JSON Schema validated')
except ImportError:
    print('INFO: jsonschema package absent, structural check only')

conn=sqlite3.connect(':memory:')
conn.execute('PRAGMA foreign_keys=ON')
conn.executescript((ROOT/'migrations/0001_init.sql').read_text())
conn.execute("INSERT INTO projects(id,title) VALUES('p1','A'),('p2','B')")
conn.execute("INSERT INTO photos(id,project_id,file_path,import_index) VALUES('a','p1','/tmp/a.jpg',1),('b','p2','/tmp/b.jpg',1),('c','p1','/tmp/c.jpg',2)")
conn.execute("INSERT INTO photo_decisions(project_id,photo_id,status) VALUES('p1','a','keep'),('p2','b','keep')")
conn.execute("INSERT INTO sequences(id,project_id) VALUES('s1','p1')")
conn.execute("INSERT INTO sequence_items(sequence_id,photo_id,position) VALUES('s1','a',0)")
try:
    conn.execute("INSERT INTO sequence_items(sequence_id,photo_id,position) VALUES('s1','b',1)")
    raise AssertionError('cross-project sequence photo unexpectedly accepted')
except sqlite3.IntegrityError as e:
    assert ('CROSS_PROJECT_SEQUENCE_PHOTO' in str(e) or 'SEQUENCE_ITEM_NOT_KEPT' in str(e)),str(e)
conn.execute("INSERT INTO drafts(id,project_id,title,base_project_revision,created_by) VALUES('d1','p1','draft',0,'agent')")
try:
    conn.execute("INSERT INTO draft_items(draft_id,photo_id,position,proposed_action) VALUES('d1','b',0,'keep')")
    raise AssertionError('cross-project draft photo unexpectedly accepted')
except sqlite3.IntegrityError as e:
    assert 'CROSS_PROJECT_DRAFT_PHOTO' in str(e),str(e)
conn.execute("INSERT INTO photo_decisions(project_id,photo_id,status) VALUES('p1','c','reject')")
try:
    conn.execute("INSERT INTO sequence_items(sequence_id,photo_id,position) VALUES('s1','c',1)")
    raise AssertionError('rejected photo unexpectedly accepted in canonical sequence')
except sqlite3.IntegrityError as e:
    assert 'SEQUENCE_ITEM_NOT_KEPT' in str(e),str(e)
try:
    conn.execute("UPDATE photo_decisions SET status='reject' WHERE photo_id='a'")
    raise AssertionError('cannot reject while still in canonical sequence')
except sqlite3.IntegrityError as e:
    assert 'REMOVE_FROM_SEQUENCE_BEFORE_REJECT' in str(e),str(e)
assert conn.execute('PRAGMA foreign_key_check').fetchall()==[]
print('OK: migration SQL, foreign keys, cross-project guards and kept-photo rule')
print('ALL PACKAGE CONTRACT CHECKS PASSED')
