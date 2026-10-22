import {createServer} from "node:http";
import {readFile,writeFile} from "node:fs/promises";
import {dirname,resolve,extname} from "node:path";
import {fileURLToPath} from "node:url";
import {execFileSync} from "node:child_process";
import {createHash} from "node:crypto";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const app=resolve(root,"../../开发文档/contact-sheet-ai-demo/between-frames");
const {chromium}=await import("playwright").catch(()=>import(app+"/node_modules/playwright/index.mjs"));
const server=createServer(async(req,res)=>{try{
 if(req.url==="/"){res.setHeader("content-type","text/html");res.end('<html><body style="margin:0;background:#071018"></body></html>');return;}
 const path=resolve(root,"."+decodeURIComponent(new URL(req.url,"http://localhost").pathname));if(!path.startsWith(root+"/"))throw Error("path");
 const b=await readFile(path);res.setHeader("content-type",extname(path)===".mp4"?"video/mp4":"application/octet-stream");res.setHeader("Accept-Ranges","bytes");
 const m=req.headers.range?.match(/^bytes=(\d+)-(\d*)$/);
 if(m){const start=Number(m[1]),end=m[2]?Math.min(Number(m[2]),b.length-1):b.length-1;res.writeHead(206,{"Content-Range":"bytes "+start+"-"+end+"/"+b.length,"Content-Length":end-start+1});res.end(b.subarray(start,end+1));}
 else{res.setHeader("Content-Length",b.length);res.end(b);}
}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,"127.0.0.1",r));const base="http://127.0.0.1:"+server.address().port;
const browser=await chromium.launch({headless:true});const page=await browser.newPage();
const results=[];
try{
 await page.goto(base);
 for(const [name,w,h] of [["横版-1920x1080",1920,1080],["竖版-1080x1920",1080,1920]]){
  const filename="FocalRoom-宣传片-"+name+".mp4",file=resolve(root,"deliverables",filename);
  const probe=JSON.parse(execFileSync("ffprobe",["-v","error","-show_format","-show_streams","-of","json",file],{encoding:"utf8"}));
  const vs=probe.streams.find(s=>s.codec_type==="video"),as=probe.streams.find(s=>s.codec_type==="audio");
  if(vs.width!==w||vs.height!==h||vs.codec_name!=="h264"||vs.pix_fmt!=="yuv420p"||as.codec_name!=="aac"||Number(probe.format.duration)!==32)throw Error("Bad media specification");
  execFileSync("ffmpeg",["-v","error","-i",file,"-f","null","-"],{stdio:["ignore","ignore","pipe"]});
  await page.evaluate(({src})=>new Promise((r,j)=>{document.body.innerHTML="";const v=document.createElement("video");v.id="check";v.muted=true;v.preload="auto";v.src=src;v.onloadeddata=()=>r();v.onerror=()=>j(Error("Video cannot load"));document.body.append(v);}),{src:base+"/deliverables/"+encodeURIComponent(filename)});
  await page.evaluate(async()=>{await document.getElementById("check").play();});
  await page.waitForFunction(()=>document.getElementById("check").currentTime>.15);
  await page.evaluate(()=>document.getElementById("check").pause());
  const seeks=[];
  for(const time of [0.2,4.95,7.4,10.95,16.95,22.95,28.2,31.6]){
   const frame=await page.evaluate(time=>new Promise((r,j)=>{const v=document.getElementById("check");v.onseeked=()=>{const c=document.createElement("canvas");c.width=v.videoWidth;c.height=v.videoHeight;const cx=c.getContext("2d");cx.drawImage(v,0,0);const a=cx.getImageData(0,0,c.width,c.height).data;let min=255,max=0;for(let i=0;i<a.length;i+=400){min=Math.min(min,a[i]);max=Math.max(max,a[i]);}r({time:v.currentTime,range:max-min,image:(time===7.4||time===31.6)?c.toDataURL("image/jpeg",.95).split(",")[1]:null});};v.onerror=()=>j(Error("seek failed"));v.currentTime=time;}),time);
   if(frame.range<50)throw Error("Empty video frame");
   if(frame.image)await writeFile(resolve(root,"qa/decoded-"+name+"-"+time+".jpg"),Buffer.from(frame.image,"base64"));
   seeks.push({requested:time,actual:frame.time,range:frame.range});
  }
  const bytes=await readFile(file);
  results.push({filename,width:w,height:h,duration:Number(probe.format.duration),frameRate:vs.r_frame_rate,videoCodec:vs.codec_name,pixelFormat:vs.pix_fmt,colorRange:vs.color_range,colorSpace:vs.color_space,audioCodec:as.codec_name,audioChannels:as.channels,audioSampleRate:as.sample_rate,fullDecodePassed:true,browserPlaybackPassed:true,seeks,bytes:bytes.length,sha256:createHash("sha256").update(bytes).digest("hex")});
 }
 const logoSource=await readFile(app+"/public/focalroom-mark.png"),logo=await readFile(root+"/assets/focalroom-mark.png");
 if(!logo.equals(logoSource))throw Error("Logo changed");
 await writeFile(resolve(root,"qa/media-verification.json"),JSON.stringify({verifiedAt:new Date().toISOString(),results,logoMatchesProduct:true},null,2));
 console.log("Both videos decode, play and seek correctly; logo matches the product.");
}finally{await browser.close();await new Promise(r=>server.close(r));}
