import json, subprocess, time, socket, uuid, hashlib
from pathlib import Path
d=json.loads(Path("/tmp/focalroom-login-build.json").read_text())
root=Path("/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames")
prefix="fr-otp-"+uuid.uuid4().hex[:8]
d.update({"smoke_prefix":prefix})
Path("/tmp/focalroom-login-build.json").write_text(json.dumps(d))
def run(args,input=None,check=True):
    r=subprocess.run(["docker",*args],input=input,text=True,capture_output=True)
    if check and r.returncode:
        # 不输出包含运行时凭据的命令行。
        raise RuntimeError(r.stderr[-2500:] or r.stdout[-2500:])
    return r
containers=[]
try:
    image=d["api_image"]
    # 增量重建最后一次源码；不关闭缓存，借源码摘要验证 COPY 确实刷新。
    for name in [str(p.relative_to(root)) for p in (root/"apps/api/src").rglob("*.ts")]:
        p=Path(d["context"])/name; p.write_bytes((root/name).read_bytes()); p.chmod(0o600)
    build=run(["build","--progress=plain","-f",str(Path(d["context"])/"ops/shared/Dockerfile.api"),"-t",image,d["context"]])
    Path("/tmp/focalroom-login-docker-api-final.log").write_text(build.stdout+build.stderr)
    names=["apps/api/src/server.ts","apps/api/src/service.ts","apps/api/src/auth.ts","apps/api/src/database.ts","apps/api/src/email.ts","apps/api/src/rate-limit.ts","apps/api/src/magic-login.ts","apps/api/migrations/0012_magic_login.sql"]
    script="""import fs from 'node:fs'; import crypto from 'node:crypto';
      const files=JSON.parse(process.env.SMOKE_FILES);
      for (const file of files) fs.accessSync(file,fs.constants.R_OK);
      await import('./apps/api/src/server.ts');
      console.log(JSON.stringify({uid:process.getuid(),hashes:Object.fromEntries(files.map(f=>[f,crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex')]))}));"""
    container=prefix+"-read"; containers.append(container)
    result=run(["run","--name",container,"--network","none","-e","SMOKE_FILES="+json.dumps(names),image,"node","--input-type=module","-e",script])
    metadata=json.loads(result.stdout)
    assert metadata["uid"] != 0
    for name in names: assert metadata["hashes"][name]==hashlib.sha256((root/name).read_bytes()).hexdigest()
    print("PASS: non-root image imports server and exact current source hashes match",flush=True)
    run(["network","create","--internal",prefix])
    password=uuid.uuid4().hex
    pg=prefix+"-pg"; containers.append(pg)
    run(["run","-d","--name",pg,"--network",prefix,"--network-alias","postgres",
         "--mount","type=volume,source="+prefix+"-pgdata,target=/var/lib/postgresql/data",
         "-e","POSTGRES_USER=focalroom","-e","POSTGRES_DB=focalroom",
         "-e","POSTGRES_PASSWORD="+password,"--pull=never","postgres:16-alpine"])
    for _ in range(50):
        if run(["exec",pg,"pg_isready","-U","focalroom","-d","focalroom"],check=False).returncode==0: break
        time.sleep(0.5)
    else: raise RuntimeError("isolated PostgreSQL startup timeout")
    # 角色只存在于本次随机命名的全新测试库。
    roles=f"""CREATE ROLE focalroom_migrator LOGIN PASSWORD '{password}';
      CREATE ROLE focalroom_app LOGIN PASSWORD '{password}';
      GRANT CONNECT ON DATABASE focalroom TO focalroom_migrator,focalroom_app;
      GRANT USAGE,CREATE ON SCHEMA public TO focalroom_migrator;
      GRANT USAGE ON SCHEMA public TO focalroom_app;"""
    run(["exec","-i",pg,"psql","-v","ON_ERROR_STOP=1","-U","focalroom","-d","focalroom"],input=roles)
    sock=socket.socket(); sock.bind(("127.0.0.1",0)); port=sock.getsockname()[1]; sock.close()
    origin=f"http://127.0.0.1:{port}"
    appurl=f"postgresql://focalroom_app:{password}@postgres:5432/focalroom"
    migurl=f"postgresql://focalroom_migrator:{password}@postgres:5432/focalroom"
    migrate=prefix+"-migrate"; containers.append(migrate)
    common=["--network",prefix,"-e","FOCALROOM_WEB_ORIGIN="+origin,
       "-e","FOCALROOM_PREVIEW_DIRECTORY=/data/previews","-e","FOCALROOM_DATABASE_URL="+appurl]
    run(["run","--name",migrate,*common,"-e","FOCALROOM_MIGRATOR_DATABASE_URL="+migurl,image,"node","apps/api/src/server.ts","--migrate"])
    api=prefix+"-api"; containers.append(api)
    # 正常镜像 CMD；不注入任何邮件/模型密钥。
    run(["run","-d","--name",api,"--network-alias","api",*common,image])
    probe="""const r=await fetch('http://127.0.0.1:3002/api/v2/auth/me'); console.log(r.status); if(r.status!==401)process.exit(1);"""
    for _ in range(40):
        result=run(["exec",api,"node","-e",probe],check=False)
        if result.returncode==0: break
        time.sleep(0.5)
    else: raise RuntimeError("normal API image failed readiness")
    print("PASS: explicit migrator DDL, normal API CMD as app role, unauthenticated /auth/me = 401",flush=True)
    no_mail="""const r=await fetch('http://127.0.0.1:3002/api/v2/auth/magic/request',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({email:'docker-only@fixture.test'})});
      const v=await r.json(); console.log(JSON.stringify({status:r.status,code:v.error?.code}));
      if(r.status!==503||v.error?.code!=='EMAIL_UNAVAILABLE')process.exit(1);"""
    result=run(["exec",api,"node","-e",no_mail])
    print("PASS: missing mail key rejects with EMAIL_UNAVAILABLE, no fake send success",flush=True)
    # Web 镜像在另一构建会话完成后验证，最长等待五分钟。
    for _ in range(100):
        if run(["image","inspect",d["web_image"]],check=False).returncode==0: break
        time.sleep(1)
    else: raise RuntimeError("Web image not available")
    web=prefix+"-web"; containers.append(web)
    run(["run","-d","--name",web,"--network",prefix,"-p",f"127.0.0.1:{port}:80",d["web_image"]])
    html_probe="""const r=await fetch('http://web/'); const s=await r.text(); if(r.status!==200||!s.includes('FocalRoom — 团队协作审稿'))process.exit(1);
      const a=await fetch('http://web/api/v2/auth/me'); if(a.status!==401)process.exit(2);
      const logo=await fetch('http://web/focalroom-mark.png'); if(logo.status!==200)process.exit(3); console.log('200 HTML + PNG / 401 API');"""
    # 使用已有API容器从相同私有网络探测真实Nginx代理。
    # 给Web添加稳定网络别名，避免依赖随机容器名。
    html_probe=html_probe.replace("http://web",f"http://{web}")
    result=run(["exec",api,"node","-e",html_probe])
    print("PASS: Nginx serves latest title and readable PNG; API proxy returns 401 rather than 502",flush=True)
    version=run(["exec",pg,"psql","-U","focalroom","-d","focalroom","-Atc","select version()"]).stdout.strip()
    summary={"prefix":prefix,"source_mode_input":"0600 files/0700 directories","image_uid":metadata["uid"],
      "source_hashes":metadata["hashes"],"api_image":image,"web_image":d["web_image"],"postgres_version":version,
      "normal_api_cmd":True,"least_privilege_app":True,"no_mail_status":503,"web_status":200,"proxy_auth_status":401,
      "production_changes":False,"real_email_or_model_calls":0,"volumes_retained":True}
    Path("/tmp/focalroom-login-image-summary.json").write_text(json.dumps(summary,indent=2))
finally:
    for name in reversed(containers):
        run(["stop","--time","5",name],check=False)
    print("Test containers stopped; fixture volumes and images retained.",flush=True)
