#!/usr/bin/env python3
"""Static schema + SQLite sample migration verification. Not a runtime E2E test."""
from pathlib import Path
import json, sqlite3, sys
root=Path(__file__).resolve().parents[1]
files=list((root/'contracts').glob('*.json'))
assert len(files)>=3
for file in files:
    data=json.loads(file.read_text(encoding='utf-8'))
    assert data and isinstance(data,dict)
    if file.name=='agent-tools.schema.json':
        names=[t['name'] for t in data['tools']]
        assert len(names)==len(set(names)) and len(names)>=12
        assert 'proposal.apply' in data['policy']['forbid_model_direct_commands']
accept=json.loads((root/'tests'/'acceptance-matrix.json').read_text(encoding='utf-8'))
assert len(accept['cases'])>=20
assert len({x['id'] for x in accept['cases']})==len(accept['cases'])
con=sqlite3.connect(':memory:')
script=(root/'db'/'local'/'0001_local.sql').read_text(encoding='utf-8')
con.executescript(script)
con.execute("INSERT INTO local_projects(id,title) VALUES('p1','Test')")
con.execute("INSERT INTO local_photos(id,project_id,source_path,content_hash) VALUES('photo1','p1','/tmp/photo.jpg','hash1')")
try:
    con.execute("INSERT INTO local_photos(id,project_id,source_path,content_hash) VALUES('photo2','not_p1','/tmp/b.jpg','hash2')")
    raise AssertionError('FK failed')
except sqlite3.IntegrityError:
    pass
try:
    import jsonschema
except ImportError:
    print('WARN jsonschema absent; only structural JSON parsing performed')
else:
    for f in (root/'contracts').glob('*.schema.json'):
        d=json.loads(f.read_text(encoding='utf-8'))
        if '$schema' in d: jsonschema.Draft202012Validator.check_schema(d)
    sch=json.loads((root/'contracts'/'command-envelope.schema.json').read_text(encoding='utf-8'))
    good={'command_id':'cmd_12345678','workspace_id':'w1','project_id':'p1','expected_revision':1,
          'kind':'sequence.move_item','payload':{'sequence_id':'s1','item_id':'i1','to_index':2},'idempotency_key':'idempotent-key-1'}
    jsonschema.validate(good,sch)
    bad=dict(good); bad['payload']={'sequence_id':'s1'}
    assert not jsonschema.Draft202012Validator(sch).is_valid(bad)
print('PASS: contracts parsed, tool names unique, acceptance index valid, local migration OK, foreign key enforcement OK')
