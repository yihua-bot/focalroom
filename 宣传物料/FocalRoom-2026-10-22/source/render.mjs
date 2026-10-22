import {createServer} from "node:http";
import {readFile,writeFile,mkdir} from "node:fs/promises";
import {resolve,dirname,extname} from "node:path";
import {fileURLToPath} from "node:url";
import {spawn} from "node:child_process";
import {once} from "node:events";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const app=resolve(root,"../../开发文档/contact-sheet-ai-demo/between-frames");
const {chromium}=await import("playwright").catch(()=>import(app+"/node_modules/playwright/index.mjs"));
const mime={".js":"text/javascript",".html":"text/html",".png":"image/png",".svg":"image/svg+xml",".wav":"audio/wav"};
const server=createServer(async(req,res)=>{try{const path=resolve(root,"."+decodeURIComponent(new URL(req.url,"http://localhost").pathname));if(!path.startsWith(root+"/"))throw Error("Invalid path");res.setHeader("content-type",mime[extname(path)]||"application/octet-stream");res.end(await readFile(path));}catch{res.writeHead(404);res.end();}});
await new Promise(r=>server.listen(0,"127.0.0.1",r));const port=server.address().port;
const browser=await chromium.launch({headless:true});
const page=await browser.newPage({viewport:{width:1920,height:1080}});
const errors=[];page.on("pageerror",e=>errors.push(e.message));
const saveCanvas=async(path,kind="image/png",quality=.96)=>{const data=await page.evaluate(({kind,quality})=>document.getElementById("film").toDataURL(kind,quality).split(",")[1],{kind,quality});await writeFile(resolve(root,path),Buffer.from(data,"base64"));};
try{
 await page.goto("http://127.0.0.1:"+port+"/source/studio.html");await page.evaluate(()=>window.ready);
 await page.evaluate(()=>window.renderPoster(true));await saveCanvas("deliverables/FocalRoom-海报-竖版-2160x3240.png");
 const jpgPortrait=await page.evaluate(()=>{const p=document.createElement("canvas");p.width=1080;p.height=1620;p.getContext("2d").drawImage(document.getElementById("film"),0,0,1080,1620);return p.toDataURL("image/jpeg",.94).split(",")[1];});
 await writeFile(resolve(root,"deliverables/FocalRoom-海报-竖版-1080x1620.jpg"),Buffer.from(jpgPortrait,"base64"));
 await page.evaluate(()=>window.renderPoster(false));await saveCanvas("deliverables/FocalRoom-海报-横版-3840x2160.png");
 const jpgLandscape=await page.evaluate(()=>{const p=document.createElement("canvas");p.width=1920;p.height=1080;p.getContext("2d").drawImage(document.getElementById("film"),0,0,1920,1080);return p.toDataURL("image/jpeg",.94).split(",")[1];});
 await writeFile(resolve(root,"deliverables/FocalRoom-海报-横版-1920x1080.jpg"),Buffer.from(jpgLandscape,"base64"));
 const times=[2.8,7.4,13.4,19.4,25.1,30.5];
 for(const portrait of [false,true])for(let i=0;i<times.length;i++){
  await page.evaluate(({t,p})=>window.renderFrame(t,p),{t:times[i],p:portrait});
  await saveCanvas("qa/"+(portrait?"portrait":"landscape")+"-"+i+".jpg","image/jpeg",.93);
 }
 await page.evaluate(times=>{
  const contact=document.createElement("canvas");contact.width=1920;contact.height=1752;const c=contact.getContext("2d");c.fillStyle="#0a1117";c.fillRect(0,0,1920,1752);
  times.forEach((t,i)=>{window.renderFrame(t,false);let x=(i%2)*960,y=Math.floor(i/2)*584;c.drawImage(document.getElementById("film"),x,y,960,540);c.font="18px 'Avenir Next'";c.fillStyle="#d4e3e8";c.fillText(t.toFixed(1)+" s",x+20,y+573);});
  window.contactData=contact.toDataURL("image/jpeg",.94).split(",")[1];
 },times);
 await writeFile(resolve(root,"qa/storyboard.jpg"),Buffer.from(await page.evaluate(()=>window.contactData),"base64"));
 console.log("Posters and 12 keyframes rendered.");
 if(process.argv.includes("--final")){
  for(const portrait of [false,true]){
   const name=portrait?"FocalRoom-宣传片-竖版-1080x1920.mp4":"FocalRoom-宣传片-横版-1920x1080.mp4";
   const output=resolve(root,"deliverables",name),log=[];
   const proc=spawn("ffmpeg",["-hide_banner","-y","-f","image2pipe","-vcodec","mjpeg","-framerate","30","-i","pipe:0","-i",resolve(root,"assets/score.wav"),"-map","0:v:0","-map","1:a:0","-c:v","libx264","-threads","4","-preset","medium","-crf","18","-vf","scale=in_range=pc:out_range=tv:in_color_matrix=bt601:out_color_matrix=bt709,format=yuv420p","-pix_fmt","yuv420p","-color_range","tv","-colorspace","bt709","-color_primaries","bt709","-color_trc","bt709","-profile:v","high","-level:v","4.2","-r","30","-af","loudnorm=I=-18:TP=-2:LRA=8","-c:a","aac","-b:a","192k","-ar","48000","-t","32","-movflags","+faststart","-metadata","title=FocalRoom — 从照片，到你的作品","-metadata","comment=Original campaign imagery and score; actual product components with illustrative project data",output],{stdio:["pipe","ignore","pipe"]});
   proc.stderr.on("data",chunk=>log.push(chunk));
   const completed=new Promise((r,j)=>{proc.on("exit",code=>code===0?r():j(Error("Video encoding failed: "+code)));proc.on("error",j);});
   for(let i=0;i<960;i++){
    const data=await page.evaluate(({t,p})=>{window.renderFrame(t,p);return document.getElementById("film").toDataURL("image/jpeg",.95).split(",")[1];},{t:i/30,p:portrait});
    if(!proc.stdin.write(Buffer.from(data,"base64")))await once(proc.stdin,"drain");
    if(i%180===0)console.log((portrait?"Portrait":"Landscape")+": "+Math.round(i/960*100)+"%");
   }
   proc.stdin.end();await completed;
   await writeFile(resolve(root,"qa/"+(portrait?"portrait":"landscape")+"-encode.log"),Buffer.concat(log));
   console.log("Finished "+name);
  }
 }
 await writeFile(resolve(root,"qa/render.json"),JSON.stringify({duration:32,framesPerSecond:30,errors,scenes:await page.evaluate(()=>window.campaignScenes),generatedAt:new Date().toISOString()},null,2));
 if(errors.length)throw Error(errors.join("\n"));
}finally{await browser.close();await new Promise(r=>server.close(r));}
