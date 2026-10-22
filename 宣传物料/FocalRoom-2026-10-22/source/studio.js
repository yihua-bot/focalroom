/* Editable campaign composition. Interface images are captured from product components. */
const canvas=document.getElementById("film"),ctx=canvas.getContext("2d",{alpha:false});
const C={bg:"#0a1117",white:"#f4f2ed",muted:"#9aa9b1",blue:"#38b9e2",edge:"#33444e"};
const images={};
const sources={art:"key-visual.png",atlas:"photo-atlas.png",logo:"focalroom-mark.png",work:"work.png",ai:"ai.png",sequence:"sequence.png",review:"review.png"};
window.ready=Promise.all(Object.entries(sources).map(async([key,name])=>{const i=new Image();i.src="/assets/"+name;await i.decode();images[key]=i;})).then(()=>document.fonts.ready);
function rect(x,y,w,h,r=0,fill=C.bg){ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.fillStyle=fill;ctx.fill();}
function line(x,y,x2,y2,color=C.edge,w=1){ctx.beginPath();ctx.moveTo(x,y);ctx.lineTo(x2,y2);ctx.strokeStyle=color;ctx.lineWidth=w;ctx.stroke();}
function text(s,x,y,size=30,color=C.white,weight=400,font="'PingFang SC',sans-serif",align="left",tracking=0){ctx.save();ctx.font=weight+" "+size+"px "+font;ctx.fillStyle=color;ctx.textBaseline="top";ctx.textAlign=align;ctx.letterSpacing=tracking+"px";ctx.fillText(s,x,y);ctx.restore();}
function block(s,x,y,size,color=C.white,leading=1.25,weight=400,font="'Songti SC',serif"){s.split("\n").forEach((v,i)=>text(v,x,y+i*size*leading,size,color,weight,font));}
function cover(im,x,y,w,h,zoom=1,fx=.5,fy=.5){const ratio=Math.max(w/im.width,h/im.height)*zoom,sw=w/ratio,sh=h/ratio;ctx.drawImage(im,(im.width-sw)*fx,(im.height-sh)*fy,sw,sh,x,y,w,h);}
function photo(id,x,y,w,h,r=0){ctx.save();ctx.beginPath();ctx.roundRect(x,y,w,h,r);ctx.clip();const aw=images.atlas.width/4,ah=images.atlas.height/3;ctx.drawImage(images.atlas,(id%4)*aw,Math.floor(id/4)*ah,aw,ah,x,y,w,h);ctx.restore();}
function logo(x,y,size){rect(x,y,size,size,size*.24,"#f2f5f5");const scale=Math.min(size*.76/images.logo.width,size*.76/images.logo.height);const w=images.logo.width*scale,h=images.logo.height*scale;ctx.drawImage(images.logo,x+(size-w)/2,y+(size-h)/2,w,h);}
function brand(x,y,size=54,word=38){logo(x,y,size);text("Focal",x+size+17,y+size*.07,word,C.white,600,"'Avenir Next',sans-serif");ctx.font="600 "+word+"px 'Avenir Next',sans-serif";const w=ctx.measureText("Focal").width;text("Room",x+size+17+w,y+size*.07,word,C.blue,600,"'Avenir Next',sans-serif");text("PHOTOGRAPHY WORKSPACE",x+size+18,y+size*.80,9,C.muted,500,"'Avenir Next',sans-serif","left",1.6);}
function bg(w,h){rect(0,0,w,h,0,C.bg);const g=ctx.createRadialGradient(w*.84,h*.26,0,w*.84,h*.26,w*.8);g.addColorStop(0,"#0f2a37");g.addColorStop(1,"#0a1117");ctx.fillStyle=g;ctx.fillRect(0,0,w,h);}
function shade(x,y,w,h,dir="horizontal",strength=.95){const g=ctx.createLinearGradient(x,y,dir==="horizontal"?x+w:x,dir==="horizontal"?y:y+h);g.addColorStop(0,"rgba(6,13,18,"+strength+")");g.addColorStop(.5,"rgba(6,13,18,.72)");g.addColorStop(1,"rgba(6,13,18,0)");ctx.fillStyle=g;ctx.fillRect(x,y,w,h);}
function frame(im,x,y,w,h,region){const bounds=region||[0,0,im.width,im.height];const scale=Math.min(w/bounds[2],h/bounds[3]);const fw=bounds[2]*scale,fh=bounds[3]*scale;x+=(w-fw)/2;y+=(h-fh)/2;w=fw;h=fh;ctx.save();ctx.shadowColor="#0009";ctx.shadowBlur=50;ctx.shadowOffsetY=20;rect(x,y,w,h,20,"#17232b");ctx.shadowBlur=0;ctx.shadowOffsetY=0;ctx.beginPath();ctx.roundRect(x+1,y+1,w-2,h-2,19);ctx.clip();if(region)ctx.drawImage(im,...region,x,y,w,h);else cover(im,x,y,w,h);ctx.restore();ctx.save();ctx.beginPath();ctx.roundRect(x,y,w,h,20);ctx.strokeStyle="#50616d88";ctx.lineWidth=1.2;ctx.stroke();ctx.restore();}
function pill(s,x,y,w,fs=20){rect(x,y,w,fs+28,50,"#122a36");text(s,x+20,y+12,fs,"#9bdaf1",500);}
const ease=x=>1-Math.pow(1-Math.max(0,Math.min(1,x)),3);
const scenes=[
 {start:0,end:4.8,title:"好照片，\n不止一张。",desc:"那些拍下的瞬间，值得成为作品。",label:"PHOTOGRAPHS → YOUR BODY OF WORK"},
 {start:4.8,end:10.8,title:"让想法，\n先被听见。",desc:"说出主题、用途与想留下的感觉。",label:"01 / AI 智能选片",key:"ai"},
 {start:10.8,end:16.8,title:"选得从容，\n留下喜欢。",desc:"比较、标记、保留。\n把注意力，还给照片。",label:"02 / 选片工作台",key:"work"},
 {start:16.8,end:22.8,title:"顺序里，\n有你的表达。",desc:"把独立的瞬间，\n编排成一个完整的故事。",label:"03 / 作品序列",key:"sequence"},
 {start:22.8,end:27.8,title:"一起看见，\n新的可能。",desc:"把评论留在照片旁，\n让讨论，围绕作品展开。",label:"04 / 协作审稿",key:"review"},
 {start:27.8,end:32,title:"从照片，\n到你的作品。",desc:"AI 选片 · 作品编排 · 协作审稿",label:"FOCALROOM"},
];
function drawLandscape(index,t){const s=scenes[index],p=(t-s.start)/(s.end-s.start),enter=ease(Math.min((t-s.start)/.95,1)),dy=(1-enter)*36;const w=1920,h=1080;
 bg(w,h);
 if(index===0||index===5){cover(images.art,0,0,w,h,1.025+.025*p,.62,.5);shade(0,0,index===5?w:w*.82,h,"horizontal",index===5?.96:.97);}
 if(index===0){brand(106,78,60,41);text(s.label,110,269+dy,15,C.blue,500,"'Avenir Next',sans-serif","left",2);block(s.title,103,330+dy,112);text(s.desc,110,671+dy,29,"#c6cfd0");line(110,811,430,811,"#74909e66");text("从照片，到你的作品",110,841,22,C.muted);text("focroom.cc",1728,1000,22,C.white,500,"'Avenir Next',sans-serif","right");return;}
 if(index===5){ctx.save();ctx.globalAlpha*=.5;rect(0,0,w,h,0,"#071018");ctx.restore();logo(904,176,112);text("Focal",958-359,350,148,C.white,600,"'Avenir Next',sans-serif");ctx.font="600 148px 'Avenir Next'";let a=ctx.measureText("Focal").width;text("Room",958-359+a,350,148,C.blue,600,"'Avenir Next',sans-serif");text("从照片，到你的作品。",960,568,70,C.white,400,"'Songti SC',serif","center");text(s.desc,960,705,29,"#bbc7cf",400,"'PingFang SC',sans-serif","center");line(805,800,1115,800,"#6a8f9e66");text("focroom.cc",960,848,44,C.blue,500,"'Avenir Next',sans-serif","center",1);return;}
 brand(100,67,48,32);text(s.label,105,259+dy,22,C.blue,500);block(s.title,98,326+dy,83);block(s.desc,105,592+dy,27,"#aebbc3",1.65,400,"'PingFang SC',sans-serif");text("focroom.cc",105,1005,20,C.muted,400,"'Avenir Next',sans-serif");
 let x=737+(1-enter)*45,y=170,wf=1080,hf=760;
 if(index===1){frame(images.ai,x,y,wf,hf,[800,0,1120,1080]);pill("AI 给建议，决定由你。",105,794,330,22);}
 if(index===2){frame(images.work,x,y,wf,hf,[200,120,1475,960]);[0,2,5].forEach((id,i)=>{let px=105+i*161,py=794-8*Math.sin(p*Math.PI+i*.4);photo(id,px,py,143,112,8);ctx.strokeStyle=i===1?C.blue:"#c9b591";ctx.lineWidth=2;ctx.strokeRect(px+2,py+2,139,108);});}
 if(index===3){frame(images.sequence,680+(1-enter)*45,180,1140,600,[205,100,1715,902]);const order=[0,2,5,7,8,11];order.forEach((id,i)=>{const reveal=ease((t-s.start-i*.11)/.7);const px=112+i*282+(1-reveal)*24,py=812+(1-reveal)*60;ctx.save();ctx.globalAlpha*=reveal;rect(px-7,py-7,250,187,6,"#e5e1d8");photo(id,px,py,236,166);text(String(i+1).padStart(2,"0"),px,py+171,11,"#15232b",600,"'Avenir Next',sans-serif");ctx.restore();});}
 if(index===4){frame(images.review,x,y,wf,hf,[700,0,1220,1080]);pill("摄影师  ↔  编辑",105,794,285,22);}
}
function drawPortrait(index,t){const s=scenes[index],p=(t-s.start)/(s.end-s.start),enter=ease(Math.min((t-s.start)/.95,1)),dy=(1-enter)*44,w=1080,h=1920;bg(w,h);
 if(index===0||index===5){cover(images.art,0,0,w,h,1.015+.02*p,.78,.5);shade(0,0,w,h,"vertical",.98);ctx.save();ctx.globalAlpha*=.15;rect(0,0,w,h,0,"#061019");ctx.restore();}
 if(index===0){brand(74,99,63,45);text("FOR THE WAY YOU SEE",78,378+dy,18,C.blue,500,"'Avenir Next',sans-serif","left",2.4);block(s.title,70,465+dy,116);block("那些拍下的瞬间，\n值得成为作品。",78,827+dy,36,"#c4cdd1",1.6,400,"'PingFang SC',sans-serif");text("focroom.cc",78,1770,32,C.white,500,"'Avenir Next',sans-serif");return;}
 if(index===5){ctx.save();ctx.globalAlpha*=.45;rect(0,0,w,h,0,"#061019");ctx.restore();logo(472,447,136);text("FocalRoom",540,657,126,C.white,600,"'Avenir Next',sans-serif","center");block(s.title,210,948,87,C.white,1.45);text("AI 选片 · 作品编排 · 协作审稿",540,1298,31,"#b7c6ce",400,"'PingFang SC',sans-serif","center");line(337,1454,743,1454,"#7399ac66");text("focroom.cc",540,1537,51,C.blue,500,"'Avenir Next',sans-serif","center",1);return;}
 brand(72,82,49,35);text(s.label,76,260+dy,25,C.blue,500);block(s.title,68,331+dy,94);block(s.desc,76,622+dy,31,"#b6c4cc",1.6,400,"'PingFang SC',sans-serif");
 const y=836+(1-enter)*50;
 const regions={1:[1155,0,765,1080],2:[210,195,1050,915],3:[210,150,1160,930],4:[855,0,1065,1080]};
 frame(images[s.key],58,y,964,807,regions[index]);
 if(index===1)pill("AI 给建议，决定由你。",76,1715,370,26);
 else if(index===2)text("保留你的判断。",76,1736,31,C.white);
 else if(index===3)text("让照片之间，产生关系。",76,1736,31,C.white);
 else text("作品的下一步，一起决定。",76,1736,31,C.white);
 text("focroom.cc",1000,1812,22,C.muted,500,"'Avenir Next',sans-serif","right");
}
window.renderFrame=(t,portrait=false)=>{canvas.width=portrait?1080:1920;canvas.height=portrait?1920:1080;let index=scenes.findIndex(s=>t>=s.start&&t<s.end);if(index<0)index=5;const elapsed=t-scenes[index].start;
 if(index>0&&elapsed<.55){(portrait?drawPortrait:drawLandscape)(index-1,scenes[index-1].end-.001);ctx.save();ctx.globalAlpha=ease(elapsed/.55);(portrait?drawPortrait:drawLandscape)(index,t);ctx.restore();}else(portrait?drawPortrait:drawLandscape)(index,t);
};
window.renderPoster=(portrait=true)=>{canvas.width=portrait?2160:3840;canvas.height=portrait?3240:2160;ctx.save();ctx.scale(2,2);const w=portrait?1080:1920,h=portrait?1620:1080;bg(w,h);
 if(portrait){brand(77,84,61,46);text("FOR THE WAY YOU SEE",82,247,17,C.blue,500,"'Avenir Next',sans-serif","left",2.3);block("从照片，",73,319,105);block("到你的作品。",73,454,105,C.blue);text("AI 选片 · 作品编排 · 协作审稿",82,614,26,"#bac9d0");cover(images.art,0,694,1080,648,1.02,.74,.5);const g=ctx.createLinearGradient(0,680,0,850);g.addColorStop(0,C.bg);g.addColorStop(1,"#0a111700");ctx.fillStyle=g;ctx.fillRect(0,680,1080,175);const g2=ctx.createLinearGradient(0,1250,0,1430);g2.addColorStop(0,"#0a111700");g2.addColorStop(1,C.bg);ctx.fillStyle=g2;ctx.fillRect(0,1250,1080,180);frame(images.work,80,1006,920,517.5);rect(80,984,150,35,18,"#143546");text("FOCALROOM",155,993,12,"#c1e6f5",600,"'Avenir Next',sans-serif","center",1.3);line(82,1556,998,1556,"#3a5663");text("让下一组照片，成为作品。",82,1576,20,"#bbc7ca");text("focroom.cc",998,1569,29,C.blue,500,"'Avenir Next',sans-serif","right");}
 else{cover(images.art,0,0,w,h,1,.5,.5);shade(0,0,1300,h,"horizontal",.98);brand(100,83,61,45);text("FOR THE WAY YOU SEE",105,265,17,C.blue,500,"'Avenir Next',sans-serif","left",2.5);block("从照片，",98,340,112);block("到你的作品。",98,489,112,C.blue);text("AI 选片 · 作品编排 · 协作审稿",106,670,29,"#c2cbd0");frame(images.work,1010,610,810,455.6);text("让下一组照片，成为作品。",106,873,25,C.muted);line(106,938,500,938,"#557686");text("focroom.cc",106,972,44,C.blue,500,"'Avenir Next',sans-serif");}
 ctx.restore();
};
window.campaignScenes=scenes;
