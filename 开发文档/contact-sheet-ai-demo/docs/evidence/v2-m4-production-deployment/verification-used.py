import json,subprocess,sys,stat,shutil,urllib.request,urllib.error,re
from pathlib import Path
root=Path("/opt/focalroom"); release=root/"releases/otp-20261018-amd64"
log=json.loads((release/"deployment-journal.json").read_text())
assert log["phase"]=="complete"
backup=Path(log["backup_directory"])
manifest=json.loads((backup/"manifest.json").read_text())
result={"journal":log,"first_attempt":json.loads((release/"attempt-1-journal.json").read_text()),"python":sys.version.split()[0],"backup":{"path":str(backup),"directory_mode":oct(backup.stat().st_mode&0o777),"environment_mode":oct((backup/"environment.env").stat().st_mode&0o777),"custom_dump_bytes":manifest["files"]["database.dump"]["bytes"],"custom_dump_sha256":manifest["files"]["database.dump"]["sha256"],"preview_archive_sha256":manifest["files"]["previews.tar.gz"]["sha256"],"preview_files":len(manifest["preview_manifest"]["files"])}}
sql="""BEGIN READ ONLY;
SELECT 'migration',version,sha256 FROM fr_migrations ORDER BY version;
SELECT 'role',rolname,rolsuper,rolcreatedb,rolcreaterole FROM pg_roles WHERE rolname IN ('focalroom_app','focalroom_migrator') ORDER BY rolname;
SELECT 'owner',pg_get_userbyid(relowner),count(*) FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind='r' GROUP BY relowner;
SELECT 'users',count(*) FROM fr_users;
SELECT 'projects',count(*) FROM fr_projects;
COMMIT;"""
p=subprocess.run(["docker","exec","-i","focalroom-shared-review-postgres-1","psql","-X","-q","-v","ON_ERROR_STOP=1","-U","focalroom","-d","focalroom","-At"],input=sql,text=True,capture_output=True,check=True)
result["db_readonly_metadata"]=p.stdout.splitlines()
result["public"]={}
for path in ["/","/api/v2/auth/me"]:
 req=urllib.request.Request("https://team.focroom.cc"+path,headers={"Cache-Control":"no-cache","Origin":"https://team.focroom.cc"})
 try:
  with urllib.request.urlopen(req,timeout=20) as response: status=response.status; body=response.read(300000)
 except urllib.error.HTTPError as error: status=error.code; body=error.read(300000)
 entry={"status":status}
 if path=="/":
  match=re.search("<title>(.*?)</title>",body.decode()); entry["title"]=match.group(1) if match else None
 else:
  try: entry["error_code"]=json.loads(body).get("error",{}).get("code")
  except (ValueError,TypeError): entry["error_code"]=None; entry["body_kind"]="non_json"
  entry["bytes"]=len(body)
 result["public"][path]=entry
print(json.dumps(result,ensure_ascii=False,indent=2),flush=True)
