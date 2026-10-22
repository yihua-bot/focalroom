
import React from "react";
import {createRoot} from "react-dom/client";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import App from "../../../开发文档/contact-sheet-ai-demo/between-frames/src/App.tsx";
import {MockRepository} from "../../../开发文档/contact-sheet-ai-demo/between-frames/src/services/MockRepository.ts";
import {I18nProvider} from "../../../开发文档/contact-sheet-ai-demo/between-frames/src/i18n/index.tsx";
import {AISelectionDrawer} from "../../../开发文档/contact-sheet-ai-demo/between-frames/src/features/ai-editor/AISelectionDrawer.tsx";
import {AIPanel} from "../../../开发文档/contact-sheet-ai-demo/between-frames/src/features/ai-editor/AIPanel.tsx";
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
