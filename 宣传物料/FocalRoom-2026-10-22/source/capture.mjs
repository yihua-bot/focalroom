import { readFile, writeFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve, relative } from "node:path";
const out = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const app = resolve(out, "../../开发文档/contact-sheet-ai-demo/between-frames");
const { createServer } = await import(app + "/node_modules/vite/dist/node/index.js");
const { default: react } = await import(app + "/node_modules/@vitejs/plugin-react/dist/index.js");
const { chromium } = await import(app + "/node_modules/playwright/index.mjs");
const src=resolve(out,"source"), assets=resolve(out,"assets");
const atlas=(await readFile(resolve(assets,"photo-atlas.png"))).toString("base64");
await mkdir(resolve(assets,"frames"),{recursive:true});
for(let i=0;i<12;i++) {
  const x=(i%4)*418,y=Math.floor(i/4)*(941/3);
  await writeFile(resolve(assets,`frames/photo-${i}.svg`),`<svg xmlns="http://www.w3.org/2000/svg" width="836" height="627" viewBox="${x} ${y} 418 ${941/3}"><image href="data:image/png;base64,${atlas}" width="1672" height="941"/></svg>`);
}
const modulePath=name=>JSON.stringify(relative(src,resolve(app,name)));
await writeFile(resolve(src,"capture.html"),'<!doctype html><html lang="zh-CN"><meta charset="utf-8"><div id="root"></div><script type="module" src="/capture.tsx"></script></html>');
await writeFile(resolve(src,"capture.tsx"),`
import React from "react";
import {createRoot} from "react-dom/client";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import App from ${modulePath("src/App.tsx")};
import {MockRepository} from ${modulePath("src/services/MockRepository.ts")};
import {I18nProvider} from ${modulePath("src/i18n/index.tsx")};
import {AISelectionDrawer} from ${modulePath("src/features/ai-editor/AISelectionDrawer.tsx")};
import {AIPanel} from ${modulePath("src/features/ai-editor/AIPanel.tsx")};
localStorage.setItem("focalroom-locale","zh");
const keep=[0,2,5,7,8,11];
const names=["潮汐","光的入口","独行","海风","浪花","远航","沿海公路","窗外","薄雾","门后的光","涟漪","午后"];
const project={id:"coast",title:"沿海，慢下来",brief:"用海边散步的照片，讲一个关于光、风和停留的故事。让人物、海面与建筑之间，留出呼吸的空间。",revision:1,
photos:names.map((name,i)=>({id:"p"+i,index:i+1,name:name+".jpg",src:"/frames/photo-"+i+".svg",status:keep.includes(i)?"keep":"undecided",note:"",rating:keep.includes(i)?4:0})),sequence:keep.map(i=>"p"+i),events:[]};
const repository=new MockRepository([project]);
const auth={status:async()=>({authenticated:true,waiting:false,origin:"https://team.focroom.cc",user:{id:"creative-demo",name:"摄影师",email:"studio@focroom.cc",avatar:null},error:null}),open:async()=>{},cancel:async()=>{},logout:async()=>true,subscribe:async()=>()=>{},updateProfile:async()=>{}};
const agentRepo={agent:{query:async()=>({tasks:[],drafts:[]}),subscribe:async()=>()=>{},aiConfig:async()=>({ready:true,keySource:"keychain",capabilities:["vision","structured_output","instruction_following","tool_calling"],observation:null,consentProvider:"demo",configurationError:null}),request:async()=>{throw new Error("宣传界面不发起模型调用")}}};
const ai=new URLSearchParams(location.search).get("scene")==="ai";
createRoot(document.getElementById("root")!).render(<I18nProvider><QueryClientProvider client={new QueryClient()}><App repository={repository} auth={auth}/>{ai&&<AISelectionDrawer projectTitle={project.title} onClose={()=>{}}>{(settingsOpen,closeSettings)=><AIPanel repository={agentRepo as any} project={project as any} selectedIds={[]} initialGoal="帮我选一组有电影感的海边故事，保留 6 张。" initialCount={6} onUpdated={()=>{}} settingsOpen={settingsOpen} onSettingsClose={closeSettings}/>}</AISelectionDrawer>}</QueryClientProvider></I18nProvider>);
`);
await writeFile(resolve(src,"web.html"),'<!doctype html><html lang="zh-CN"><meta charset="utf-8"><div id="root"></div><script type="module" src="/web.tsx"></script></html>');
await writeFile(resolve(src,"web.tsx"),`
import React from "react";import {createRoot} from "react-dom/client";import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import SharedApp from ${modulePath("apps/web/src/SharedApp.tsx")};import {WebI18nProvider} from ${modulePath("apps/web/src/i18n.tsx")};
localStorage.setItem("focalroom-locale","zh");createRoot(document.getElementById("root")!).render(<QueryClientProvider client={new QueryClient()}><WebI18nProvider><SharedApp/></WebI18nProvider></QueryClientProvider>);
`);
const server=await createServer({configFile:false,root:src,publicDir:assets,plugins:[react()],resolve:{alias:{react:app+"/node_modules/react","react-dom":app+"/node_modules/react-dom","@tanstack/react-query":app+"/node_modules/@tanstack/react-query","@betweenframes/contracts":app+"/packages/contracts/src/index.ts"}},server:{host:"127.0.0.1",port:1446,strictPort:true,fs:{allow:[resolve(out,"../..")]}}});
await server.listen();
const browser=await chromium.launch({headless:true});
try{
 const page=await browser.newPage({viewport:{width:1920,height:1080},deviceScaleFactor:1,locale:"zh-CN"});
 const errors=[];page.on("pageerror",e=>errors.push(e.message));
 await page.route("**/*",route=>{const u=new URL(route.request().url());return ["127.0.0.1","localhost"].includes(u.hostname)||u.protocol==="data:"||u.protocol==="blob:"?route.continue():route.abort();});
 for(const scene of ["work","ai"]) {
  await page.goto("http://127.0.0.1:1446/capture.html?scene="+scene+"#/project/coast");
  await page.getByTestId("photo-1").waitFor();
  if(scene==="work")await page.getByTestId("photo-1").getByRole("button",{name:/^查看第/}).click();
  if(scene==="ai")await page.getByRole("textbox",{name:"选片消息"}).waitFor();
  await page.evaluate(async()=>{await document.fonts.ready;await Promise.all([...document.images].map(i=>i.decode().catch(()=>{})));});
  await page.screenshot({path:resolve(assets,scene+".png"),animations:"disabled"});
 }
 await page.goto("http://127.0.0.1:1446/capture.html?scene=work#/project/coast");
 await page.getByRole("button",{name:"作品序列",exact:true}).click();
 await page.getByTestId("sequence-card").first().waitFor();
 await page.evaluate(async()=>{await document.fonts.ready;await Promise.all([...document.images].map(i=>i.decode().catch(()=>{})));});
 await page.screenshot({path:resolve(assets,"sequence.png"),animations:"disabled"});
 const keep=[0,2,5,7,8,11],names=["潮汐","光的入口","独行","海风","浪花","远航","沿海公路","窗外","薄雾","门后的光","涟漪","午后"];
 const snapshot={project_id:"coast",workspace_id:"studio",title:"沿海，慢下来",state:"SHARED",revision:3,cursor:"3",snapshot:{photos:names.map((name,i)=>({id:"p"+i,filename:name+".jpg",source_sha256:"a".repeat(64),selection:keep.includes(i)?"KEEP":"UNDECIDED"})),sequences:[{id:"main",title:"沿海 · 作品序列",items:keep.map(i=>({id:"item"+i,photo_id:"p"+i}))}]},principal:{role:"owner",can_approve:true,can_edit_formal:true},previews:names.map((_,i)=>({photo_id:"p"+i,content_sha256:"a".repeat(64),width:836,height:627})),comments:[{id:"c1",author_id:"editor",reviewer:"编辑",body:"让海面成为开篇，人物留到第二张，故事会更舒展。",anchor:{type:"photo",photo_id:"p0",x:0.5,y:0.5},parent_id:null,state:"open"},{id:"c2",author_id:"creator",reviewer:"摄影师",body:"保留海风和窗边的呼应，最后用午后的暖色收尾。",anchor:{type:"photo",photo_id:"p0",x:0.7,y:0.5},parent_id:null,state:"open"}],proposals:[],grants:[]};
 await page.route("**/api/v2/**",async route=>{
  const p=new URL(route.request().url()).pathname;
  if(p.endsWith("/snapshot"))return route.fulfill({json:snapshot});
  if(p.endsWith("/auth/me"))return route.fulfill({json:{id:"creator",name:"摄影师",email:"studio@focroom.cc",csrf:"demo-local",emailVerified:true}});
  if(p.endsWith("/ticket")){const id=p.split("/").at(-2);return route.fulfill({json:{url:"/frames/photo-"+id.slice(1)+".svg"}});}
  if(p.endsWith("/ws-ticket"))return route.fulfill({json:{token:"local-visual-only"}});
  return route.fulfill({json:[]});
 });
 await page.routeWebSocket("**/api/v2/realtime",ws=>{ws.onMessage(()=>{ws.send(JSON.stringify({type:"sync"}));ws.send(JSON.stringify({type:"presence",users:[{id:"creator",name:"摄影师",photo_id:"p0"},{id:"editor",name:"编辑",photo_id:"p0"}]}));});});
 await page.goto("http://127.0.0.1:1446/web.html#project=coast");
 await page.getByText("沿海，慢下来",{exact:true}).first().waitFor();
 await page.waitForFunction(()=>[...document.images].filter(i=>i.src.startsWith("blob:")&&i.naturalWidth>0).length>=6);
 await page.evaluate(async()=>{await document.fonts.ready;await Promise.all([...document.images].map(i=>i.decode().catch(()=>{})));});
 await page.screenshot({path:resolve(assets,"review.png"),animations:"disabled"});
 await writeFile(resolve(out,"qa/capture.json"),JSON.stringify({interfaceSource:"current product React components; isolated marketing project, demonstration account and review data; no real AI requests or production requests",errors},null,2));
 if(errors.length)throw new Error(errors.join("\n"));
 console.log("Captured current workbench, AI drawer, sequence and Web review.");
}finally{await browser.close();await server.close();}
