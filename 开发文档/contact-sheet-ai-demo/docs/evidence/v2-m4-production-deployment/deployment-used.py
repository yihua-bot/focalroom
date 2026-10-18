import os,sys,json,subprocess,hashlib,tarfile,shutil,stat,time,re,fcntl,urllib.parse,urllib.request,urllib.error
from pathlib import Path
os.umask(0o077)
root=Path("/opt/focalroom"); release=root/"releases/otp-20261018-amd64"
project="focalroom-shared-review"; pgname=project+"-postgres-1"
api_image="focalroom-release-api:otp-20261018-amd64"; web_image="focalroom-release-web:otp-20261018-amd64"
expected_images={api_image:"sha256:dbf00eca3ba2b9329fb822757b9cba37294835f4d4b962e7ed1439ec726f57d5",web_image:"sha256:5cc4b4dbed8dc64bc427c097e870e70a96cc13ddd26134054a2944b3e11b2933"}
archive_size=119562736; archive_sha="21d14f1345ae00c6fff428004bb9ccc6740e820feaeaeda60b009afc5012b9b7"
expected_old={"api":"sha256:acc93b7f086fc8b6c9b64ef0bf2c342f433f69c541763ce6ddc7adb35dc3a1c6","web":"sha256:b2c5dd710749a75d86b8aa77464a70caaff25acd4bf84a43d31e9c5c2876c8aa","postgres":"sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777"}
expected_migrations={"0001":"bee9cec8bb2b90dbe20538fbb7de84acd8f5aae4d33810dc1e61bcbb2b4b5d04","0002":"613949828f2508cb097bdf45a7f80505ef5296fd11d33642fab2caaf5620e7ae"}
expected_tables=sorted("fr_acl_audit fr_agent_scopes fr_commands fr_comments fr_drafts fr_events fr_grants fr_invitations fr_memberships fr_migrations fr_previews fr_projects fr_proposals fr_sessions fr_tickets fr_users fr_versions fr_workspaces".split())
log={"approved":True,"phase":"initial","production_changes":[],"checks":{}}
def emit(event,**data):
 print(json.dumps({"event":event,**data},ensure_ascii=False),flush=True)
def run(args,input=None,env=None,stdout=None,timeout=90):
 p=subprocess.run(args,input=input,stdout=stdout if stdout is not None else subprocess.PIPE,stderr=subprocess.PIPE,text=stdout is None,env=env,timeout=timeout)
 if p.returncode: raise RuntimeError("command failed: "+args[0]+" / exit "+str(p.returncode))
 return p.stdout if stdout is None else None
def inspect(name): return json.loads(run(["docker","inspect",name]))[0]
def sql(query):
 return run(["docker","exec","-i",pgname,"psql","-X","-q","-v","ON_ERROR_STOP=1","-U","focalroom","-d","focalroom","-At"],input=query)
def stream_digest(stream):
 sha=hashlib.sha256()
 for chunk in iter(lambda:stream.read(1024*1024),b""): sha.update(chunk)
 return sha.hexdigest()
def manifest_volume(path):
 files=[]; dirs=[]
 for p in sorted(path.rglob("*")):
  s=p.lstat(); name=p.relative_to(path).as_posix()
  if stat.S_ISDIR(s.st_mode): dirs.append(name)
  elif stat.S_ISREG(s.st_mode):
   with p.open("rb") as stream: sha=stream_digest(stream)
   files.append({"path":name,"bytes":s.st_size,"sha256":sha,"uid":s.st_uid,"gid":s.st_gid,"mode":oct(s.st_mode&0o777)})
  else: raise RuntimeError("preview volume contains unexpected link or special file")
 return {"files":files,"directories":dirs}
def save():
 if release.exists():
  p=release/"deployment-journal.json"; tmp=release/"deployment-journal.tmp"; tmp.write_text(json.dumps(log,ensure_ascii=False,indent=2)+"\n"); tmp.replace(p)
def request(url,method="GET",payload=None,origin=True):
 headers={"Cache-Control":"no-cache"}
 if origin: headers["Origin"]="https://team.focroom.cc"
 if payload is not None: headers["Content-Type"]="application/json"
 req=urllib.request.Request(url,data=json.dumps(payload).encode() if payload is not None else None,headers=headers,method=method)
 try:
  with urllib.request.urlopen(req,timeout=12) as response: return response.status,response.read(200000)
 except urllib.error.HTTPError as error: return error.code,error.read(200000)
lock=(root/".otp-deploy.lock").open("a")
fcntl.flock(lock.fileno(),fcntl.LOCK_EX|fcntl.LOCK_NB)
try:
 assert os.uname().machine=="x86_64"
 resume=False
 if release.exists():
  previous=json.loads((release/"deployment-journal.json").read_text())
  assert previous["phase"]=="backup" and previous["failure"]["kind"]=="AttributeError" and previous["production_changes"]==[],"unexpected release state; refuse retry"
  assert (release/"backup/environment.env").read_bytes()==(root/".env").read_bytes()
  assert not (release/"images.tar.gz").exists() and not (release/"backup-verified").exists()
  assert not (release/"attempt-1-journal.json").exists()
  shutil.copyfile(release/"deployment-journal.json",release/"attempt-1-journal.json"); resume=True
 assert hashlib.sha256((root/"ops/shared/compose.yaml").read_bytes()).hexdigest()=="7d0ab8f486bda6f44c6173d64f01b0b270c15c6c6221a8e081f7d557f982e3fb"
 ids=run(["docker","ps","-aq","--filter","label=com.docker.compose.project="+project]).split()
 containers={inspect(cid)["Config"]["Labels"]["com.docker.compose.service"]:inspect(cid) for cid in ids}
 assert set(containers)=={"api","web","postgres"}
 for name,v in containers.items(): assert v["Image"]==expected_old[name],name+" changed"
 assert not containers["api"]["State"]["Running"] and containers["web"]["State"]["Running"] and containers["postgres"]["State"]["Running"]
 assert containers["api"]["State"]["ExitCode"]==1
 preview=next(m for m in containers["api"]["Mounts"] if m["Destination"]=="/data/previews")
 pgmount=next(m for m in containers["postgres"]["Mounts"] if m["Destination"]=="/var/lib/postgresql/data")
 assert preview["Name"]=="focalroom-shared-review_preview-data" and pgmount["Name"]=="focalroom-shared-review_pg-data"
 preview_path=Path(preview["Source"]); assert preview_path.stat().st_uid==1000
 before_volume=manifest_volume(preview_path)
 assert sql("BEGIN READ ONLY; SELECT version||'|'||sha256 FROM fr_migrations ORDER BY version; COMMIT;").splitlines()==[k+"|"+v for k,v in sorted(expected_migrations.items())]
 assert sql("SELECT count(*) FROM fr_users; SELECT count(*) FROM fr_projects;").splitlines()==["0","0"]
 tables=sql("SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='r' ORDER BY relname;").splitlines(); assert tables==expected_tables
 owners=sql("SELECT DISTINCT pg_get_userbyid(relowner) FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind IN ('r','S');").splitlines(); assert owners==["focalroom"]
 assert sql("SELECT count(*) FROM pg_roles WHERE rolname='focalroom_migrator';").strip()=="0"
 assert sql("SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='S' ORDER BY relname;").splitlines()==["fr_acl_audit_id_seq","fr_events_id_seq"]
 assert sql("SELECT proname||'|'||pg_get_userbyid(proowner) FROM pg_proc WHERE pronamespace='public'::regnamespace ORDER BY proname;").splitlines()==["fr_immutable|focalroom","fr_require_envelope|focalroom"]
 assert sql("SELECT rolsuper,rolcreatedb,rolcreaterole FROM pg_roles WHERE rolname='focalroom_app';").strip()=="f|f|f"
 cfg_env={k:v for k,v in os.environ.items() if not k.startswith("FOCALROOM_")}
 base=["docker","compose","--project-name",project,"--env-file",str(root/".env"),"-f",str(root/"ops/shared/compose.yaml")]
 cfg=json.loads(run(base+["config","--format","json"],env=cfg_env))
 assert set(cfg["services"])=={"api","postgres","web"}
 api_env=cfg["services"]["api"]["environment"]
 app=urllib.parse.urlsplit(api_env["FOCALROOM_DATABASE_URL"]); migrator=urllib.parse.urlsplit(api_env["FOCALROOM_MIGRATOR_DATABASE_URL"])
 for u,user in [(app,"focalroom_app"),(migrator,"focalroom_migrator")]:
  assert u.username==user and u.hostname=="postgres" and u.path=="/focalroom" and u.password
 app_pw=urllib.parse.unquote(app.password); mig_pw=urllib.parse.unquote(migrator.password)
 assert not any(c in app_pw+mig_pw for c in "\n\r\0")
 assert not api_env.get("FOCALROOM_RESEND_API_KEY")
 env_text=(root/".env").read_text()
 assert not any(line.split("=",1)[0].strip()=="FOCALROOM_RESEND_API_KEY" and line.split("=",1)[1].strip().strip("\"'") for line in env_text.splitlines() if "=" in line and not line.lstrip().startswith("#"))
 volume_bytes=sum(f["bytes"] for f in before_volume["files"])
 assert shutil.disk_usage(root).free >= 1024**3+archive_size+700*1024**2+volume_bytes*2,"insufficient reserved disk capacity"
 if not resume: release.mkdir(parents=True,mode=0o700)
 backup=release/("backup-verified" if resume else "backup"); backup.mkdir(mode=0o700)
 log["resumed_after_python_hash_compatibility_fix"]=resume
 log["phase"]="backup"; log["before"]={"containers":{k:{"id":v["Id"],"image":v["Image"],"status":v["State"]["Status"]} for k,v in containers.items()},"migration_sha":expected_migrations,"tables":tables,"preview_files":len(before_volume["files"]),"preview_bytes":volume_bytes}
 shutil.copyfile(root/".env",backup/"environment.env"); (backup/"environment.env").chmod(0o600)
 shutil.copyfile(root/"ops/shared/compose.yaml",backup/"compose.yaml")
 with (backup/"database.dump").open("xb") as stream:
  run(["docker","exec",pgname,"pg_dump","-U","focalroom","-d","focalroom","--format=custom"],stdout=stream)
 with (backup/"database.dump").open("rb") as stream:
  p=subprocess.run(["docker","exec","-i",pgname,"pg_restore","--list"],stdin=stream,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  assert p.returncode==0
 with tarfile.open(backup/"previews.tar.gz","w:gz") as tf:
  tf.add(preview_path,arcname="previews",recursive=False)
  for name in before_volume["directories"]: tf.add(preview_path/name,arcname="previews/"+name,recursive=False)
  for item in before_volume["files"]: tf.add(preview_path/item["path"],arcname="previews/"+item["path"],recursive=False)
 assert manifest_volume(preview_path)==before_volume
 backup_manifest={"preview_manifest":before_volume,"migrations":expected_migrations,"users":0,"projects":0,"files":{}}
 for name in ["database.dump","previews.tar.gz","environment.env","compose.yaml"]:
  f=backup/name
  with f.open("rb") as stream: sha=stream_digest(stream)
  backup_manifest["files"][name]={"bytes":f.stat().st_size,"sha256":sha}
 (backup/"manifest.json").write_text(json.dumps(backup_manifest,ensure_ascii=False,indent=2)+"\n")
 log["checks"]["backup_custom_dump_readable"]=True; log["checks"]["preview_backup_unchanged"]=True
 log["backup_directory"]=str(backup); save(); emit("backup_complete",preview_files=len(before_volume["files"]),database_bytes=(backup/"database.dump").stat().st_size)
 log["phase"]="receive_archive"; save()
 archive=release/"images.tar.gz"; sha=hashlib.sha256(); remaining=archive_size
 with archive.open("xb") as stream:
  while remaining:
   chunk=sys.stdin.buffer.read(min(1024*1024,remaining))
   if not chunk: raise RuntimeError("release archive transfer incomplete")
   stream.write(chunk); sha.update(chunk); remaining-=len(chunk)
 assert not sys.stdin.buffer.read(1),"unexpected extra archive payload"
 assert sha.hexdigest()==archive_sha,"release archive SHA mismatch"
 emit("archive_verified",bytes=archive_size,sha256=archive_sha)
 log["phase"]="load_images"; save()
 run(["docker","load","-i",str(archive)],timeout=240)
 for tag,digest in expected_images.items():
  v=json.loads(run(["docker","image","inspect",tag]))[0]; assert v["Id"]==digest and v["Architecture"]=="amd64"
 assert shutil.disk_usage(root).free>=1024**3
 probe="import fs from 'node:fs'; await import('./apps/api/src/server.ts'); if(process.getuid()!==1000)process.exit(1); fs.accessSync('apps/api/migrations/0012_magic_login.sql',fs.constants.R_OK); console.log('nonroot_import_ok');"
 assert "nonroot_import_ok" in run(["docker","run","--rm","--network","none",api_image,"node","--input-type=module","-e",probe])
 log["checks"]["fixed_image_ids_and_nonroot_import"]=True; save(); emit("images_validated",disk_free_bytes=shutil.disk_usage(root).free)
 # 密码仅经 stdin 交给已有 PG 容器，不进入命令参数或输出。
 def role_auth(user,password):
  query="SELECT current_user,has_schema_privilege(current_user,'public','CREATE');"
  command='IFS= read -r PGPASSWORD; export PGPASSWORD; exec psql -X -h 127.0.0.1 -U '+user+' -d focalroom -Atc '+__import__("shlex").quote(query)
  return run(["docker","exec","-i",pgname,"sh","-c",command],input=password+"\n").strip()
 assert role_auth("focalroom_app",app_pw)=="focalroom_app|f"
 log["phase"]="role_upgrade"; save()
 password_literal="E'"+mig_pw.replace("\\","\\\\").replace("'","''")+"'"
 role_sql="""BEGIN;
 SELECT pg_advisory_xact_lock(759362109);
 CREATE ROLE focalroom_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOINHERIT PASSWORD """+password_literal+""";
 GRANT CONNECT ON DATABASE focalroom TO focalroom_migrator,focalroom_app;
 GRANT USAGE,CREATE ON SCHEMA public TO focalroom_migrator;
 GRANT USAGE ON SCHEMA public TO focalroom_app;
 DO $$ DECLARE item record; BEGIN
  FOR item IN SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='r' AND left(relname,3)='fr_' LOOP
   EXECUTE format('ALTER TABLE public.%I OWNER TO focalroom_migrator',item.relname);
  END LOOP;
  FOR item IN SELECT relname FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='S' AND left(relname,3)='fr_' LOOP
   EXECUTE format('ALTER SEQUENCE public.%I OWNER TO focalroom_migrator',item.relname);
  END LOOP;
  FOR item IN SELECT oid::regprocedure AS signature FROM pg_proc WHERE pronamespace='public'::regnamespace AND left(proname,3)='fr_' LOOP
   EXECUTE format('ALTER FUNCTION %s OWNER TO focalroom_migrator',item.signature);
  END LOOP;
 END $$;
 COMMIT;
 """
 sql(role_sql); assert role_auth("focalroom_migrator",mig_pw)=="focalroom_migrator|t"
 log["production_changes"].append("created migrator and transferred scoped FocalRoom ownership"); save(); emit("migration_role_ready")
 override=release/"deployment-override.yaml"
 override.write_text("""services:
  api:
    image: focalroom-release-api:otp-20261018-amd64
    environment:
      FOCALROOM_WEB_ORIGIN: https://team.focroom.cc
      FOCALROOM_RESEND_API_KEY: ${FOCALROOM_RESEND_API_KEY:-}
      FOCALROOM_EMAIL_FROM: ${FOCALROOM_EMAIL_FROM:-FocalRoom <noreply@focroom.cc>}
  web:
    image: focalroom-release-web:otp-20261018-amd64
""")
 lines=env_text.splitlines(keepends=True); matches=[i for i,l in enumerate(lines) if l.split("=",1)[0].strip()=="FOCALROOM_WEB_ORIGIN"]
 assert len(matches)<=1
 if matches: lines[matches[0]]="FOCALROOM_WEB_ORIGIN=https://team.focroom.cc\n"
 else: lines.append("\nFOCALROOM_WEB_ORIGIN=https://team.focroom.cc\n")
 temp=root/".env.otp-update"; assert not temp.exists(); temp.write_text("".join(lines)); temp.chmod(0o600); temp.replace(root/".env")
 compose=base+["-f",str(override)]
 resolved=json.loads(run(compose+["config","--format","json"],env=cfg_env))
 assert resolved["services"]["api"]["image"]==api_image and resolved["services"]["web"]["image"]==web_image
 assert resolved["services"]["api"]["environment"]["FOCALROOM_WEB_ORIGIN"]=="https://team.focroom.cc"
 assert not resolved["services"]["api"]["environment"].get("FOCALROOM_RESEND_API_KEY")
 assert resolved["services"]["web"]["ports"][0]["host_ip"]=="127.0.0.1" and not resolved["services"]["postgres"].get("ports")
 log["production_changes"].append("set exact team origin, env mode0600 and fixed image override"); log["phase"]="migrate"; save()
 run(compose+["run","--rm","--no-deps","--pull","never","api","node","apps/api/src/server.ts","--migrate"],env=cfg_env,timeout=120)
 versions=sql("SELECT version FROM fr_migrations ORDER BY version;").splitlines()
 assert versions==["0001","0002","0006","0008","0009","0010","0011","0012"]
 assert sql("SELECT count(*) FROM fr_users; SELECT count(*) FROM fr_projects;").splitlines()==["0","0"]
 log["migration_versions"]=versions; log["production_changes"].append("applied six registered migrations"); log["phase"]="replace_services"; save(); emit("migrations_complete",versions=versions)
 run(compose+["up","-d","--no-deps","--no-build","--pull","never","api","web"],env=cfg_env,timeout=120)
 log["production_changes"].append("recreated only API and Web"); log["phase"]="verify"; save()
 for _ in range(40):
  try:
   status,body=request("http://127.0.0.1:8084/api/v2/auth/me")
   if status==401: break
  except (OSError,urllib.error.URLError): pass
  time.sleep(0.5)
 else: raise RuntimeError("new API proxy not ready")
 status,html=request("http://127.0.0.1:8084/"); assert status==200 and "FocalRoom — 团队协作审稿" in html.decode()
 status,logo=request("http://127.0.0.1:8084/focalroom-mark.png"); assert status==200 and logo.startswith(b"\x89PNG")
 status,body=request("http://127.0.0.1:8084/api/v2/auth/magic/request","POST",{"email":"deployment-check@fixture.test"})
 assert status==503 and json.loads(body)["error"]["code"]=="EMAIL_UNAVAILABLE"
 assert sql("SELECT count(*) FROM fr_users; SELECT count(*) FROM fr_projects;").splitlines()==["0","0"]
 assert manifest_volume(preview_path)==before_volume
 after={k:inspect(project+"-"+k+"-1") for k in ["api","web","postgres"]}
 assert after["api"]["Image"]==expected_images[api_image] and after["web"]["Image"]==expected_images[web_image]
 assert after["postgres"]["Id"]==containers["postgres"]["Id"]
 assert all(x["State"]["Running"] for x in after.values())
 env_after=dict(x.split("=",1) for x in after["api"]["Config"]["Env"] if "=" in x)
 assert env_after["FOCALROOM_WEB_ORIGIN"]=="https://team.focroom.cc" and not env_after.get("FOCALROOM_RESEND_API_KEY")
 log["after"]={"containers":{k:{"id":v["Id"],"image":v["Image"],"status":v["State"]["Status"]} for k,v in after.items()},"env_mode":oct((root/".env").stat().st_mode&0o777),"disk_free_bytes":shutil.disk_usage(root).free,"users":0,"projects":0}
 log["checks"].update({"local_html_title_200":True,"local_logo_png_200":True,"local_api_anonymous_401":True,"local_no_mail_503":True,"pg_container_retained":True,"previews_unchanged":True,"no_real_mail_or_model_calls":True})
 log["phase"]="complete"; save(); emit("deployment_complete",release=str(release),checks=log["checks"],after=log["after"])
except BaseException as error:
 log["failure"]={"phase":log["phase"],"kind":type(error).__name__}
 save(); emit("deployment_stopped",phase=log["phase"],error_kind=type(error).__name__)
 # 不输出可能包含凭据的异常文本/命令，不擅自回滚已升级 schema 或启动旧 API。
 raise SystemExit(1)
finally:
 fcntl.flock(lock.fileno(),fcntl.LOCK_UN)
