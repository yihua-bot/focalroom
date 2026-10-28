import { expect,it } from 'vitest';
import { publicationRequestSchema,parsePublicationReceipt,type PublicationCommandRequest } from './publication';
it('页面顺序不可重复，文字格式受限且新增内容须有稳定编号',()=>{
  const action={type:'AddText',data:{textId:crypto.randomUUID(),pageId:crypto.randomUUID(),xMm:15,yMm:15,widthMm:100,heightMm:20,kind:'text',text:'<b>原样文字</b>',fontFamily:'serif',fontSizePt:12,lineHeight:1.4,align:'left',bold:false,color:'#112233'}};
  const wrapped=(value:unknown)=>({...request,operations:[{opId:crypto.randomUUID(),action:value}]});
  expect(publicationRequestSchema.safeParse(wrapped(action)).success).toBe(true);
  for(const data of [{...action.data,kind:'html'},{...action.data,fontFamily:'url(https://example.com)'},{...action.data,color:'red'},{...action.data,textId:''}])expect(publicationRequestSchema.safeParse(wrapped({...action,data})).success).toBe(false);
  const page=crypto.randomUUID();expect(publicationRequestSchema.safeParse(wrapped({type:'ReorderPages',data:{pageIds:[page,page]}})).success).toBe(false);
});
const request:PublicationCommandRequest={schemaVersion:3,commandId:crypto.randomUUID(),idempotencyKey:'publication-save',projectId:'demo-first-walk',publicationId:crypto.randomUUID(),baseRevision:2,operations:[{opId:crypto.randomUUID(),action:{type:'CaptureVersion',data:{label:'确认版'}}}]};
it('出版命令严格绑定版本、操作编号和独立确认，拒绝任意节点内容',()=>{
  expect(publicationRequestSchema.parse(request)).toEqual(request);
  for(const invalid of [{...request,actor:'human'},{...request,operations:[request.operations[0],request.operations[0]]},{...request,baseRevision:0},{...request,operations:[{...request.operations[0],action:{type:'CaptureVersion',data:{label:'确认版',nodes:[]}}}]}])expect(publicationRequestSchema.safeParse(invalid).success).toBe(false);
});
it('保存回执核对项目、出版物、基础版本与业务结果，不把传输成功当保存成功',()=>{
  const receipt={commandId:request.commandId,projectId:request.projectId,publicationId:request.publicationId,baseRevision:2,revision:3,changed:true,results:[null]};
  expect(parsePublicationReceipt(request,receipt)).toEqual(receipt);
  for(const invalid of [{...receipt,publicationId:crypto.randomUUID()},{...receipt,projectId:'other'},{...receipt,baseRevision:1},{...receipt,revision:4},{...receipt,changed:false,revision:2},{...receipt,results:[{}]},{...receipt,results:[]}])expect(()=>parsePublicationReceipt(request,invalid)).toThrow();
});

it('母版操作绑定版本与明确覆盖范围，不接受任意版面或混入其他写入',()=>{
  const pageId=crypto.randomUUID(),data={masterId:crypto.randomUUID(),masterRevision:1,pageIds:[pageId],instanceId:crypto.randomUUID(),overwriteLocal:false};
  const operation={opId:crypto.randomUUID(),action:{type:'ApplyMaster',data}};
  const wrapped=(value:unknown)=>({...request,operations:[{...operation,action:{type:'ApplyMaster',data:value}}]});
  expect(publicationRequestSchema.safeParse(wrapped(data)).success).toBe(true);
  for(const invalid of [{...data,pageIds:[pageId,pageId]},{...data,masterRevision:0},{...data,overwriteLocal:undefined},{...data,frames:[]},{...data,pageIds:[]}])expect(publicationRequestSchema.safeParse(wrapped(invalid)).success).toBe(false);
  expect(publicationRequestSchema.safeParse({...request,operations:[operation,request.operations[0]]}).success).toBe(false);
});

it('文字位置操作只允许几何，不能混入正文、样式或错误页面编号',()=>{
  const data={textId:crypto.randomUUID(),pageId:crypto.randomUUID(),xMm:15,yMm:20,widthMm:80,heightMm:25};const wrapped=(value:unknown)=>({...request,operations:[{opId:crypto.randomUUID(),action:{type:'UpdateTextGeometry',data:value}}]});expect(publicationRequestSchema.safeParse(wrapped(data)).success).toBe(true);for(const invalid of [{...data,text:'替换正文'},{...data,styleJson:'{}'},{...data,widthMm:0},{...data,pageId:'other'}])expect(publicationRequestSchema.safeParse(wrapped(invalid)).success).toBe(false);
});

it('跨页操作只携带稳定照片框、实体页和几何，不接受来源替换或与其他操作混合',()=>{
  const geometry={xMm:15,yMm:20,widthMm:350,heightMm:220,fitMode:'fill',cropX:.3,cropY:.7,cropZoom:1.6},data={frameId:crypto.randomUUID(),leftPageId:crypto.randomUUID(),rightPageId:crypto.randomUUID(),geometry},wrapped=(value:unknown)=>({...request,operations:[{opId:crypto.randomUUID(),action:value}]});
  const action={type:'SetSpreadFrame',data};expect(publicationRequestSchema.safeParse(wrapped(action)).success).toBe(true);
  for(const bad of [{...data,assetId:'foreign'},{...data,geometry:{...geometry,source:{}}},{...data,geometry:{...geometry,cropZoom:0}},{...data,leftPageId:'wrong'}])expect(publicationRequestSchema.safeParse(wrapped({...action,data:bad})).success).toBe(false);
  expect(publicationRequestSchema.safeParse({...wrapped(action),operations:[{opId:crypto.randomUUID(),action},request.operations[0]]}).success).toBe(false);
});
it('两页新增与成组开本更新允许同一人工事务，操作编号仍须唯一',()=>{
  const page={widthMm:210,heightMm:297,marginMm:15,bleedMm:3},first=crypto.randomUUID(),second=crypto.randomUUID(),operations=[{opId:crypto.randomUUID(),action:{type:'AddPage',data:{...page,pageId:first,afterPageId:null}}},{opId:crypto.randomUUID(),action:{type:'AddPage',data:{...page,pageId:second,afterPageId:first}}}];expect(publicationRequestSchema.safeParse({...request,operations}).success).toBe(true);expect(publicationRequestSchema.safeParse({...request,operations:operations.map(op=>({...op,opId:operations[0].opId}))}).success).toBe(false);
});

it('图层命令只允许引用和展示状态，拒绝图像来源、正文、控制字符或混入几何操作',()=>{
 const node={id:crypto.randomUUID(),kind:'frame',parentId:null,name:'封面',visible:true,locked:false},wrapped=(data:unknown)=>({...request,operations:[{opId:crypto.randomUUID(),action:{type:'SetPublicationLayers',data}}]});
 expect(publicationRequestSchema.safeParse(wrapped({nodes:[node]})).success).toBe(true);
 for(const bad of [{...node,assetId:'foreign'},{...node,contentJson:'replacement'},{...node,id:'unknown'},{...node,visible:'true'},{...node,name:'坏\u0085名字'},{...node,name:'坏\u0000名字'}])expect(publicationRequestSchema.safeParse(wrapped({nodes:[bad]})).success).toBe(false);
 const single=wrapped({nodes:[node]});expect(publicationRequestSchema.safeParse({...single,operations:[...single.operations,request.operations[0]]}).success).toBe(false);
});

it('页面网格只接收严格本页参考设置，拒绝来源和几何混入，保存须单独确认',()=>{
 const data={pageId:crypto.randomUUID(),settings:{columns:3,rows:4,columnGapMm:5,rowGapMm:4,verticalGuidesMm:[25],horizontalGuidesMm:[30]}},wrapped=(value:unknown)=>({...request,operations:[{opId:crypto.randomUUID(),action:{type:'SetPageGuides',data:value}}]});expect(publicationRequestSchema.safeParse(wrapped(data)).success).toBe(true);
 for(const bad of [{...data,source:'foreign'},{...data,pageId:'wrong'},{...data,settings:{...data.settings,columns:1.5}},{...data,settings:{...data.settings,verticalGuidesMm:[20,20]}},{...data,settings:{...data.settings,source:'foreign'}}])expect(publicationRequestSchema.safeParse(wrapped(bad)).success).toBe(false);
 const one=wrapped(data);expect(publicationRequestSchema.safeParse({...one,operations:[...one.operations,request.operations[0]]}).success).toBe(false);
});

it('母版网格可选字段不改旧请求形状，拒绝null、越界参考线及伪造来源',async()=>{
 const {savePublicationMasterSchema,publicationMasterSchema,publicationMasterLinkSchema}=await import('./publication-master');const pageId=crypto.randomUUID(),old={masterId:crypto.randomUUID(),name:'母版',sourcePageId:pageId,expectedMasterRevision:null,includeText:false};expect(JSON.stringify(savePublicationMasterSchema.parse(old))).toBe(JSON.stringify(old));expect(savePublicationMasterSchema.safeParse({...old,includeGuides:null}).success).toBe(false);expect(savePublicationMasterSchema.safeParse({...old,includeGuides:true}).success).toBe(true);
 const definition={id:old.masterId,revision:1,name:'母版',sourcePageId:pageId,widthMm:190,heightMm:257,marginMm:15,bleedMm:3,frames:[],texts:[]},settings={columns:3,rows:4,columnGapMm:5,rowGapMm:4,verticalGuidesMm:[25],horizontalGuidesMm:[30]};expect(JSON.stringify(publicationMasterSchema.parse(definition))).toBe(JSON.stringify(definition));expect(publicationMasterSchema.safeParse({...definition,guides:settings}).success).toBe(true);for(const guides of [null,{...settings,source:'foreign'},{...settings,verticalGuidesMm:[200]},{...settings,columnGapMm:100}])expect(publicationMasterSchema.safeParse({...definition,guides}).success).toBe(false);
 const link={id:old.masterId,revision:1,signature:'a'.repeat(64),frames:[],texts:[]};expect(JSON.stringify(publicationMasterLinkSchema.parse(link))).toBe(JSON.stringify(link));expect(publicationMasterLinkSchema.safeParse({...link,guideSignature:null}).success).toBe(false);expect(publicationMasterLinkSchema.safeParse({...link,guideSignature:'b'.repeat(64)}).success).toBe(true);
});
