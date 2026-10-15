import subprocess,tempfile,json,uuid,time,os,hashlib
from pathlib import Path
from PIL import Image
root=Path(tempfile.mkdtemp(prefix='bf-m1-fixes-probe-'))
p=subprocess.Popen(['src-tauri/target-v1/debug/m1-harness',str(root),'standard'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,env={**os.environ,'BETWEEN_FRAMES_OPENAI_API_KEY':''})
seq=0;out={}
def rpc(command,args=None):
 global seq
 seq+=1;p.stdin.write(json.dumps({'id':seq,'command':command,'args':args or {}})+'\n');p.stdin.flush()
 reply=json.loads(p.stdout.readline())
 if not reply['ok']:raise RuntimeError(reply['error'])
 return reply['value']
def edit(project,title):
 cid=str(uuid.uuid4())
 return {'commandId':cid,'projectId':project['id'],'expectedProjectRevision':project['revision'],'causationId':cid,'idempotencyKey':cid,'command':{'type':'UpdateProject','title':title,'brief':''}}
try:
 project=rpc('create_project',{'title':'review fix','brief':''})
 a=edit(project,'A');first=rpc('execute_gui_command',{'request':a})
 b=edit(first['data']['project'],'B');rpc('execute_gui_command',{'request':b})
 replay=rpc('execute_gui_command',{'request':a})
 assert first['data']==replay['data']
 out['receipt']={'firstRevision':first['data']['revision'],'replayRevision':replay['data']['revision'],'replayProjectRevision':replay['data']['project']['revision'],'replayTitle':replay['data']['project']['title'],'sameResult':True}
 large=root/'fixtures/oversized.dng';Image.new('L',(7000,5000),128).save(large,format='TIFF')
 before=hashlib.sha256(large.read_bytes()).hexdigest()
 token=rpc('choose_import',{'projectId':project['id']})
 job=rpc('import_photos',{'projectId':project['id'],'selectionToken':token})
 while rpc('get_import_job',{'jobId':job})['status']=='running':time.sleep(.1)
 photo=next(x for x in rpc('get_project',{'projectId':project['id']})['photos'] if x['name']=='oversized.dng')
 source=rpc('get_original_photo',{'projectId':project['id'],'photoId':photo['id']})
 with Image.open(source['path']) as image:dimensions=list(image.size)
 assert source['kind']=='derived_preview' and max(dimensions)<=1400
 assert hashlib.sha256(large.read_bytes()).hexdigest()==before
 out['raw']={'sourceDimensions':[7000,5000],'returnedDimensions':dimensions,'returnedKind':source['kind'],'sourceHashUnchanged':True}
 Path('/tmp/bf-m1-fixes-probes.json').write_text(json.dumps(out,ensure_ascii=False,indent=2)+'\n')
 print(json.dumps(out,ensure_ascii=False))
finally:
 p.stdin.close();p.wait(timeout=10)
