import { cleanup,render,screen,waitFor,within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import type { PublicationDocument,PublicationFrameUpdate,PublicationRepository } from '../../domain/publication';
import { setCopyLocale } from '../../app/copy';
import { PublicationStudio } from './PublicationStudio';
import { publicationPageOrderPreview } from './publicationPageOrder';

beforeEach(()=>setCopyLocale('zh'));afterEach(()=>{cleanup();setCopyLocale('zh');});
const publication={id:'book',projectId:'p',pubType:'photobook',title:'光与停顿',state:'draft',revision:2,createdAt:'2026-10-25'};
const doc:PublicationDocument={revision:2,layout:{schemaVersion:1,publication:{id:'book',projectId:'p',title:'光与停顿',pubType:'photobook',specJson:'{}',state:'draft'},pages:[{id:'page',pageNumber:0,side:'cover',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,frames:[{id:'frame',xMm:15,yMm:15,widthMm:160,heightMm:227,fitMode:'fit',cropX:0.5,cropY:0.5,cropZoom:1,assetId:null,renditionId:null,source:null}],texts:[]}]},previews:[{frameId:'frame'}]};
const version={id:'version',publicationId:'book',revision:1,label:'确认版',kind:'saved' as const,snapshotSha256:'a'.repeat(64),createdAt:'2026-10-25'};
function setup(overrides:Partial<PublicationRepository>={},readOnly=false){
  let current=structuredClone(doc);
  const repository={list:vi.fn(),create:vi.fn(),generate:vi.fn(),pages:vi.fn(),document:vi.fn(async()=>structuredClone(current)),versions:vi.fn(async()=>[version]),version:vi.fn(async()=>({version,layout:structuredClone(doc.layout),previews:doc.previews})),editFrame:vi.fn(async(_p:string,_b:string,_rev:number,frame:PublicationFrameUpdate)=>{current={...current,revision:current.revision+1};current.layout.pages[0].frames[0]={...current.layout.pages[0].frames[0],...frame};}),saveVersion:vi.fn().mockResolvedValue(undefined),restoreVersion:vi.fn().mockResolvedValue(undefined),pendingCommands:vi.fn(()=>[]),retryPending:vi.fn().mockResolvedValue(undefined),...overrides};
  const onClose=vi.fn(),onChanged=vi.fn().mockResolvedValue(undefined);const props={repository,projectId:'p',publication,readOnly,onClose,onChanged};
  const view=render(<PublicationStudio {...props}/>);return {...view,props,repository,onClose,onChanged};
}
it('修改先在页面预览，保存才携带当前版本与明确照片框数据',async()=>{
  const {repository}=setup();const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));
  const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'150');
  expect(repository.editFrame).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'选择照片框 1'})).toHaveStyle({width:`${150/190*100}%`});
  await user.click(screen.getByRole('button',{name:'保存调整'}));expect(await screen.findByRole('status',{name:'版面状态'})).toHaveTextContent('版面调整已保存');
  expect(repository.editFrame).toHaveBeenCalledExactlyOnceWith('p','book',2,expect.objectContaining({frameId:'frame',widthMm:150,assetId:null,renditionId:null}));expect(repository.saveVersion).not.toHaveBeenCalled();
});
it('历史只读，明确确认恢复才写入，恢复使用当前版本而非旧版本号',async()=>{
  const {repository}=setup();const user=userEvent.setup();await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(await screen.findByRole('button',{name:/^确认版/}));
  expect(await screen.findByText('查看历史版面')).toBeVisible();expect(repository.restoreVersion).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'恢复此版本'}));const confirmation=screen.getByRole('group',{name:'确认恢复'});expect(repository.restoreVersion).not.toHaveBeenCalled();
  await user.click(within(confirmation).getByRole('button',{name:'确认恢复'}));await waitFor(()=>expect(repository.restoreVersion).toHaveBeenCalledExactlyOnceWith('p','book',2,'version'));expect(await screen.findByRole('status',{name:'版面状态'})).toHaveTextContent('已恢复为新的版面版本');
});
it('结果不明时阻止新保存，确认只重试原记录，不重复编辑',async()=>{
  let pending=false;
  const editFrame=vi.fn(async()=>{pending=true;throw Object.assign(new Error('上次保存结果尚未确认'),{code:'PUBLICATION_PENDING'});});
  const retryPending=vi.fn(async()=>{pending=false;});const {repository}=setup({editFrame,retryPending,pendingCommands:()=>pending?[{publicationId:'book'}]:[]});
  const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'150');await user.click(screen.getByRole('button',{name:'保存调整'}));
  expect(await screen.findByRole('button',{name:'确认上次保存'})).toBeEnabled();expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();await user.click(screen.getByRole('button',{name:'确认上次保存'}));await waitFor(()=>expect(retryPending).toHaveBeenCalledExactlyOnceWith('p','book'));expect(editFrame).toHaveBeenCalledTimes(1);expect(repository.saveVersion).not.toHaveBeenCalled();
});
it('关闭前确认未保存调整，只读界面不提供恢复写入',async()=>{
  const {onClose,repository}=setup();const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'150');await user.click(screen.getByRole('button',{name:'关闭对话框'}));expect(onClose).not.toHaveBeenCalled();
  const group=screen.getByRole('group',{name:'放弃尚未保存的调整？'});await user.click(within(group).getByRole('button',{name:'放弃调整'}));expect(onClose).toHaveBeenCalledOnce();expect(repository.editFrame).not.toHaveBeenCalled();
  cleanup();setup({},true);await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));expect(screen.getByRole('spinbutton',{name:'宽度（mm）'})).toBeDisabled();await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(screen.getByRole('button',{name:/^确认版/}));expect(await screen.findByText('查看历史版面')).toBeVisible();expect(screen.queryByRole('button',{name:'恢复此版本'})).not.toBeInTheDocument();
});
it('项目切换立即隔离旧界面，迟到结果不能覆盖新项目',async()=>{
  let finish!:(value:PublicationDocument)=>void;const {repository,props,rerender}=setup({document:vi.fn((project)=>project==='p'?new Promise<PublicationDocument>(resolve=>{finish=resolve;}):Promise.resolve({...structuredClone(doc),layout:{...doc.layout,publication:{...doc.layout.publication,projectId:'other',id:'new',title:'另一份作品'}}})),versions:vi.fn().mockResolvedValue([])});
  await waitFor(()=>expect(repository.document).toHaveBeenCalled());rerender(<PublicationStudio {...props} projectId="other" publication={{...publication,id:'new',projectId:'other',title:'另一份作品'}}/>);expect(screen.queryByText('光与停顿')).not.toBeInTheDocument();await screen.findByRole('button',{name:'选择照片框 1'});finish(doc);await waitFor(()=>expect(screen.getByText('另一份作品')).toBeVisible());expect(screen.queryByText('光与停顿')).not.toBeInTheDocument();
});
it('英语日语韩语显示完整界面文案，不暴露内部翻译键',async()=>{
  for(const [locale,label]of [['en','Layout and versions'],['ja','レイアウトとバージョン'],['ko','레이아웃과 버전']]){setCopyLocale(locale);setup({},true);expect(await screen.findByRole('dialog',{name:label})).toBeVisible();expect(document.body.textContent).not.toMatch(/publication[A-Z]/);cleanup();}
});

it('重叠内容可从本页内容选择，未保存时不能切换；文字仍须明确保存',async()=>{
  const source=structuredClone(doc);source.layout.pages[0].texts=[{id:'text',xMm:15,yMm:15,widthMm:160,heightMm:200,contentJson:JSON.stringify({kind:'text',text:'覆盖照片的文字'}),styleJson:null}];
  const {repository}=setup({document:vi.fn().mockResolvedValue(source),editText:vi.fn()});const user=userEvent.setup();
  const objects=await screen.findByRole('combobox',{name:'本页内容'});await user.selectOptions(objects,'frame:frame');expect(screen.getByRole('spinbutton',{name:'宽度（mm）'})).toHaveValue(160);
  const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'150');expect(objects).toBeDisabled();expect(repository.editFrame).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'放弃调整'}));await user.selectOptions(objects,'text:text');await user.click(screen.getByRole('button',{name:'编辑文字'}));const editor=screen.getByRole('dialog',{name:'编辑文字'});expect(within(editor).getByRole('textbox',{name:'文字内容'})).toHaveValue('覆盖照片的文字');expect(repository.editText).not.toHaveBeenCalled();
});

it('文字选中后可键盘移动和保存几何，内容编辑需另行打开，不重写内容与样式',async()=>{
  const source=structuredClone(doc);source.layout.pages[0].texts=[{id:'text',xMm:15,yMm:20,widthMm:80,heightMm:25,contentJson:'{"kind":"text","text":"保留原文字","extra":[1]}',styleJson:'{"fontSizePt":14,"custom":true}'}];
  const editTextGeometry=vi.fn().mockResolvedValue(undefined),editText=vi.fn();setup({document:vi.fn().mockResolvedValue(source),editTextGeometry,editText});const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择文字框 1'}));expect(screen.queryByRole('dialog',{name:'编辑文字'})).toBeNull();await user.keyboard('{ArrowRight}');expect(screen.getByRole('spinbutton',{name:'左侧位置（mm）'})).toHaveValue(16);expect(editTextGeometry).not.toHaveBeenCalled();expect(screen.getByRole('combobox',{name:'选择页面'})).toBeDisabled();await user.click(screen.getByRole('button',{name:'保存调整'}));await waitFor(()=>expect(editTextGeometry).toHaveBeenCalledExactlyOnceWith('p','book',2,{textId:'text',pageId:'page',xMm:16,yMm:20,widthMm:80,heightMm:25}));expect(editText).not.toHaveBeenCalled();
});

it('页面缩放只改变查看方式，未保存调整在放大和适合窗口之间保持',async()=>{
  const {repository}=setup();const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'150');await user.selectOptions(screen.getByRole('combobox',{name:'页面缩放'}),'100');expect(screen.getByRole('figure',{name:'封面'})).toHaveStyle({width:`${190*96/25.4}px`});expect(width).toHaveValue(150);expect(screen.getByText('尚未保存')).toBeVisible();expect(repository.editFrame).not.toHaveBeenCalled();await user.selectOptions(screen.getByRole('combobox',{name:'页面缩放'}),'fit');expect((screen.getByRole('figure',{name:'封面'}) as HTMLElement).style.width).toBe('');expect(width).toHaveValue(150);expect(repository.editFrame).not.toHaveBeenCalled();
});

it('Shift选择照片与文字，整组键盘移动只预览，明确保存后一次提交并保留选择',async()=>{
  let current=structuredClone(doc);current.layout.pages[0].texts=[{id:'text',xMm:30,yMm:110,widthMm:40,heightMm:20,contentJson:'{"kind":"text","text":"原文字","extra":true}',styleJson:'{"custom":true}'}];
  const arrangeObjects=vi.fn(async(_p:string,_b:string,_r:number,frames:PublicationFrameUpdate[],texts:import('../../domain/publication').PublicationTextGeometry[])=>{for(const frame of frames)current.layout.pages[0].frames[0]={...current.layout.pages[0].frames[0],...frame};for(const text of texts)current.layout.pages[0].texts[0]={...current.layout.pages[0].texts[0],...text};current={...current,revision:3};});
  setup({arrangeObjects,document:vi.fn(async()=>structuredClone(current))});const user=userEvent.setup();await user.click(await screen.findByRole('button',{name:'选择照片框 1'}));await user.keyboard('{Shift>}');await user.click(screen.getByRole('button',{name:'选择文字框 1'}));await user.keyboard('{/Shift}');expect(screen.getByText('已选 2 项')).toBeVisible();expect(screen.getByRole('button',{name:'选择照片框 1'})).toHaveAttribute('aria-pressed','true');expect(screen.getByRole('button',{name:'选择文字框 1'})).toHaveAttribute('aria-pressed','true');await user.keyboard('{ArrowRight}');expect(arrangeObjects).not.toHaveBeenCalled();expect(screen.getByRole('combobox',{name:'选择页面'})).toBeDisabled();await user.click(screen.getByRole('button',{name:'保存调整'}));await waitFor(()=>expect(arrangeObjects).toHaveBeenCalledExactlyOnceWith('p','book',2,[expect.objectContaining({frameId:'frame',xMm:16})],[{textId:'text',pageId:'page',xMm:31,yMm:110,widthMm:40,heightMm:20}]));expect(screen.getByText('已选 2 项')).toBeVisible();expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
});

it('默认浏览页面，未保存调整阻止跳页并显示原因；非法版面不会提交',async()=>{
  const many=structuredClone(doc);many.layout.pages.push({...structuredClone(doc.layout.pages[0]),id:'page-2',pageNumber:1});
  const {repository}=setup({document:vi.fn().mockResolvedValue(many)});const user=userEvent.setup();
  await screen.findByRole('button',{name:'选择照片框 1'});
  expect(screen.getByRole('tab',{name:'页面'})).toHaveAttribute('aria-selected','true');
  await user.click(screen.getByRole('button',{name:'下一页版面'}));expect(screen.getByRole('combobox',{name:'选择页面'})).toHaveValue('page-2');
  await user.click(screen.getByRole('button',{name:'选择照片框 1'}));
  const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'2000');
  expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();expect(screen.getByRole('button',{name:'上一页版面'})).toBeDisabled();
  expect(screen.getByRole('alert')).toHaveTextContent('请检查位置');expect(screen.getByText('尚未保存')).toBeVisible();expect(screen.getByText('保存或放弃调整后，可切换页面。')).toBeVisible();
  await user.click(screen.getByRole('button',{name:'放弃调整'}));await user.click(screen.getByRole('button',{name:'上一页版面'}));expect(screen.getByRole('combobox',{name:'选择页面'})).toHaveValue('page');expect(repository.editFrame).not.toHaveBeenCalled();
});

it('页面缩略图和版本导航支持键盘，保存命名版本后显示版本记录',async()=>{
  const {repository}=setup();const user=userEvent.setup();await screen.findByRole('button',{name:'封面'});
  expect(document.querySelector('.publication-page-thumb button')).toBeNull();
  screen.getByRole('tab',{name:'页面'}).focus();await user.keyboard('{ArrowRight}');expect(screen.getByRole('tab',{name:'项目照片'})).toHaveFocus();await user.keyboard('{ArrowRight}');expect(screen.getByRole('tab',{name:'图层'})).toHaveFocus();await user.keyboard('{ArrowRight}');expect(screen.getByRole('tab',{name:'版本记录'})).toHaveFocus();
  await user.keyboard('{ArrowLeft}');expect(screen.getByRole('tab',{name:'图层'})).toHaveFocus();await user.keyboard('{ArrowLeft}');expect(screen.getByRole('tab',{name:'项目照片'})).toHaveFocus();await user.keyboard('{ArrowLeft}');expect(screen.getByRole('tab',{name:'页面'})).toHaveAttribute('aria-selected','true');
  await user.type(screen.getByRole('textbox',{name:'版本名称'}),'交付前');await user.click(screen.getByRole('button',{name:'保存版本'}));
  await waitFor(()=>expect(repository.saveVersion).toHaveBeenCalledWith('p','book',2,'交付前'));expect(screen.getByRole('tab',{name:'版本记录'})).toHaveAttribute('aria-selected','true');
});

it('历史比较按稳定页面身份，不按后来变化的页码错配照片；移除的页明确提示',async()=>{
  const current=structuredClone(doc),cover=current.layout.pages[0];
  const stable={...structuredClone(cover),id:'stable-page',pageNumber:1,side:'right',frames:[{...cover.frames[0],id:'stable-frame',widthMm:100}]};
  const other={...structuredClone(cover),id:'other-page',pageNumber:2,side:'left',frames:[{...cover.frames[0],id:'other-frame',widthMm:120}]};
  current.layout.pages=[cover,stable,other];
  const historical={...structuredClone(current.layout),pages:[cover,{...other,pageNumber:1,side:'right'},{...stable,pageNumber:2,side:'left'}]};
  const user=userEvent.setup();setup({document:vi.fn().mockResolvedValue(current),version:vi.fn().mockResolvedValue({version,layout:historical,previews:[]})});
  await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(await screen.findByRole('button',{name:/^确认版/}));await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'stable-page');await user.click(screen.getByRole('button',{name:'与当前并排比较'}));
  expect(screen.getByRole('figure',{name:'当前版面 · 第 1 页'}).querySelector('.publication-frame')).toHaveStyle({width:`${100/190*100}%`});expect(screen.getByRole('figure',{name:'确认版 · 第 2 页'})).toBeVisible();
  cleanup();current.layout.pages=[cover,other];setup({document:vi.fn().mockResolvedValue(current),version:vi.fn().mockResolvedValue({version,layout:historical,previews:[]})});
  await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(await screen.findByRole('button',{name:/^确认版/}));await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'stable-page');await user.click(screen.getByRole('button',{name:'与当前并排比较'}));
  expect(screen.getByText('这页在当前版本中已移除。')).toBeVisible();expect(screen.getAllByRole('figure')).toHaveLength(1);
});

it('跨页中点击右页照片会编辑右页，未保存时不会切到另一页',async()=>{
  const current=structuredClone(doc),base=current.layout.pages[0];current.layout.pages.push({...structuredClone(base),id:'left',pageNumber:2,side:'left',frames:[{...base.frames[0],id:'left-frame',widthMm:100}]},{...structuredClone(base),id:'right',pageNumber:3,side:'right',frames:[{...base.frames[0],id:'right-frame',widthMm:120}]});
  setup({document:vi.fn().mockResolvedValue(current)});const user=userEvent.setup();await screen.findByRole('combobox',{name:'选择页面'});await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'left');await user.click(screen.getByRole('button',{name:'跨页预览'}));
  await user.click(within(screen.getByRole('figure',{name:'第 3 页'})).getByRole('button',{name:'选择照片框 1'}));expect(screen.getByRole('combobox',{name:'选择页面'})).toHaveValue('right');const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});expect(width).toHaveValue(120);await user.clear(width);await user.type(width,'50');
  await user.click(screen.getByRole('figure',{name:'第 2 页'}));expect(screen.getByRole('combobox',{name:'选择页面'})).toHaveValue('right');expect(screen.getByRole('button',{name:'保存调整'})).toBeEnabled();
});

it('当前版与同身份历史页并排时保持独立组件，不产生重复标识',async()=>{
  const messages=vi.spyOn(console,'error');try{
    setup();const user=userEvent.setup();await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(await screen.findByRole('button',{name:/^确认版/}));await user.click(screen.getByRole('button',{name:'与当前并排比较'}));
    expect(document.querySelectorAll('.publication-studio-papers .publication-studio-paper')).toHaveLength(2);expect(messages.mock.calls.filter(args=>/same key|Encountered two children/.test(args.join(' ')))).toHaveLength(0);
  }finally{messages.mockRestore();}
});

it('项目照片先填空框预览，明确确认才写入；取消、重复和未保存保护不重排旧内容',async()=>{
  const photo={id:'photo-new',index:1,name:'新照片.jpg',src:'photo.jpg',rating:0,note:'',status:'keep' as const,width:4000,height:3000};
  let current=structuredClone(doc);current.layout.pages[0].frames.push({...current.layout.pages[0].frames[0],id:'existing',assetId:'old-asset',renditionId:'old-render',cropZoom:1.8});
  const arrangeFrames=vi.fn(async(_p:string,_b:string,_r:number,_updates:unknown[],_adds:unknown[],assignments:Array<{frameId:string}>=[])=>{current={...current,revision:3};current.layout.pages[0].frames[0]={...current.layout.pages[0].frames[0],assetId:'new-asset'};current.previews=[{frameId:assignments[0].frameId,photoId:photo.id}];});
  const {props,rerender}=setup({document:vi.fn(async()=>structuredClone(current)),arrangeFrames});rerender(<PublicationStudio {...props} photos={[photo]} sequencePhotoIds={[photo.id]}/>);const user=userEvent.setup();await screen.findByRole('button',{name:'选择照片框 1'});await user.click(screen.getByRole('tab',{name:'项目照片'}));await user.click(screen.getByRole('button',{name:'放置 新照片.jpg'}));expect(screen.getByRole('group',{name:'照片放置预览'})).toBeVisible();expect(arrangeFrames).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'放置 新照片.jpg'})).toBeDisabled();await user.click(screen.getByRole('button',{name:'取消放置'}));expect(arrangeFrames).not.toHaveBeenCalled();await user.click(screen.getByRole('button',{name:'放置 新照片.jpg'}));await user.click(screen.getByRole('button',{name:'确认放置'}));await waitFor(()=>expect(arrangeFrames).toHaveBeenCalledExactlyOnceWith('p','book',2,[],[],[{frameId:'frame',photoId:photo.id,renditionId:null}]));expect(await screen.findByRole('status',{name:'版面状态'})).toHaveTextContent('照片已排入本页');expect(screen.getByRole('button',{name:'放置 新照片.jpg'})).toBeDisabled();expect(current.layout.pages[0].frames[1]).toMatchObject({assetId:'old-asset',renditionId:'old-render',cropZoom:1.8});
});

it('页序先预览且可继续浏览和放弃，保存只发送完整身份；封面没有排序把手',async()=>{
  let current=structuredClone(doc);current.layout.pages.push(...[1,2,3].map(number=>({...structuredClone(doc.layout.pages[0]),id:'p'+number,pageNumber:number,side:number%2?'right':'left',frames:[]})));
  const reorderPages=vi.fn(async(_p:string,_b:string,_r:number,ids:string[])=>{current={...current,revision:3,layout:{...current.layout,pages:publicationPageOrderPreview(current.layout.pages,ids)!}};});setup({document:vi.fn(async()=>structuredClone(current)),reorderPages});const user=userEvent.setup();await screen.findByRole('button',{name:'封面'});expect(screen.queryByRole('button',{name:'拖动排序 封面'})).toBeNull();await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'p3');await user.click(screen.getByRole('button',{name:'向前移一页'}));expect(reorderPages).not.toHaveBeenCalled();expect(screen.getByRole('group',{name:'页序预览'})).toBeVisible();expect(screen.getByRole('combobox',{name:'选择页面'})).toHaveValue('p3');await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'p1');expect(screen.getByRole('group',{name:'页序预览'})).toBeVisible();await user.click(screen.getByRole('button',{name:'放弃排序'}));expect(reorderPages).not.toHaveBeenCalled();await user.selectOptions(screen.getByRole('combobox',{name:'选择页面'}),'p3');await user.click(screen.getByRole('button',{name:'向前移一页'}));await user.click(screen.getByRole('button',{name:'保存顺序'}));await waitFor(()=>expect(reorderPages).toHaveBeenCalledExactlyOnceWith('p','book',2,['page','p1','p3','p2']));expect(await screen.findByRole('status',{name:'版面状态'})).toHaveTextContent('页面顺序已保存');expect(current.layout.pages.map(page=>page.id)).toEqual(['page','p1','p3','p2']);expect(screen.queryByRole('group',{name:'页序预览'})).toBeNull();
});

it('未保存图层状态时仍可在列表选择和分组；页面排序不可覆盖图层预览',async()=>{
 const {continuousFixture}=await import('../../test/publicationContinuousFixture');const source=continuousFixture().layout,first=source.pages[0],frame=first.frames[0],text={id:crypto.randomUUID(),xMm:20,yMm:230,widthMm:80,heightMm:20,contentJson:'{"kind":"text","text":"层内文字"}',styleJson:null};first.texts=[text];
 const setLayers=vi.fn().mockResolvedValue(undefined),reorderPages=vi.fn(),user=userEvent.setup();setup({document:vi.fn().mockResolvedValue({revision:3,layout:source,previews:[]}),setLayers,reorderPages});
 await user.click(await screen.findByRole('tab',{name:'图层'}));await screen.findByRole('button',{name:'隐藏 照片框'});await user.click(screen.getByRole('button',{name:'隐藏 照片框'}));await user.click(screen.getByRole('button',{name:'显示 照片框'}));
 const row=document.querySelector('[data-publication-layer-id="'+frame.id+'"]')!;await user.click(row.querySelector('.publication-layer-select')!);await user.keyboard('{Shift>}');await user.click(screen.getByRole('button',{name:'选择图层 层内文字'}));await user.keyboard('{/Shift}');expect(screen.getByRole('button',{name:'分组'})).toBeEnabled();await user.click(screen.getByRole('button',{name:'分组'}));
 await user.click(screen.getByRole('tab',{name:'页面'}));expect(screen.queryByRole('button',{name:/^拖动排序/})).not.toBeInTheDocument();expect(reorderPages).not.toHaveBeenCalled();await user.click(screen.getByRole('tab',{name:'图层'}));await user.click(screen.getByRole('button',{name:'保存图层'}));
 expect(setLayers).toHaveBeenCalledOnce();const nodes=setLayers.mock.calls[0][3].nodes,group=nodes.find((node:{kind:string})=>node.kind==='group');expect(nodes.filter((node:{parentId:string|null})=>node.parentId===group.id).map((node:{id:string})=>node.id).sort()).toEqual([frame.id,text.id].sort());
});

it('整组移除先显示前后对比，取消与名称输入退格不写入，删除键也必须确认',async()=>{
 const {continuousFixture}=await import('../../test/publicationContinuousFixture'),{publicationLayers,publicationChangeLayers,publicationLayerPreview}=await import('../../domain/publicationLayers');
 const source=continuousFixture().layout,page=source.pages[0],text={id:crypto.randomUUID(),xMm:20,yMm:210,widthMm:80,heightMm:20,contentJson:'{"kind":"text","text":"组内文字"}',styleJson:null};page.texts=[text];const groupId=crypto.randomUUID(),grouped=publicationLayerPreview(source,publicationChangeLayers(source,publicationLayers(source),{type:'group',id:groupId,ids:[page.frames[0].id,text.id],name:'封面组合'}));
 const removeObjects=vi.fn().mockResolvedValue(undefined),user=userEvent.setup();setup({document:vi.fn().mockResolvedValue({revision:7,layout:grouped,previews:[]}),setLayers:vi.fn(),removeObjects,arrangeObjects:vi.fn()});
 await user.click(await screen.findByRole('tab',{name:'图层'}));await user.click(await screen.findByRole('button',{name:'选择图层 封面组合'}));await user.click(screen.getByRole('button',{name:'移除整组内容…'}));
 let dialog=screen.getByRole('dialog',{name:'确认移除整组内容'});expect(dialog).toHaveTextContent('1 个照片框、1 个文字框');expect(within(dialog).getByRole('heading',{name:'移除前'})).toBeVisible();expect(within(dialog).getByRole('heading',{name:'移除后'})).toBeVisible();expect(dialog.querySelectorAll('.publication-frame,.publication-text')).toHaveLength(2);expect(removeObjects).not.toHaveBeenCalled();await user.click(within(dialog).getByRole('button',{name:'返回'}));
 const name=screen.getByRole('textbox',{name:'图层名称'});await user.click(name);await user.keyboard('{End}{Backspace}合');expect(screen.queryByRole('dialog',{name:'确认移除整组内容'})).toBeNull();expect(removeObjects).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button',{name:'选择图层 封面组合'}));await user.keyboard('{Delete}');dialog=screen.getByRole('dialog',{name:'确认移除整组内容'});expect(removeObjects).not.toHaveBeenCalled();await user.click(within(dialog).getByRole('button',{name:'确认移除整组内容'}));await waitFor(()=>expect(removeObjects).toHaveBeenCalledExactlyOnceWith('p','book',7,expect.arrayContaining([{kind:'frame',id:page.frames[0].id},{kind:'text',id:text.id}])));expect(removeObjects.mock.calls[0][3]).toHaveLength(2);
});

it('跨页翻页跳过整组可见页面，键盘一致且未保存时阻止跳页',async()=>{
  const current=structuredClone(doc),cover=current.layout.pages[0];
  current.layout.pages=[cover,...[1,2,3,4,5].map(n=>({...structuredClone(cover),id:'page-'+n,pageNumber:n,side:n%2?'right':'left',frames:[{...cover.frames[0],id:'frame-'+n}]})),{...structuredClone(cover),id:'back',pageNumber:-1,side:'back',frames:[]}];
  const {repository}=setup({document:vi.fn().mockResolvedValue(current)});const user=userEvent.setup();
  const choose=await screen.findByRole('combobox',{name:'选择页面'});
  await user.selectOptions(choose,'page-2');await user.click(screen.getByRole('button',{name:'跨页预览'}));
  await user.click(screen.getByRole('button',{name:'下一跨页'}));expect(choose).toHaveValue('page-4');
  await user.click(screen.getByRole('button',{name:'上一跨页'}));expect(choose).toHaveValue('page-3');
  expect(screen.getByRole('figure',{name:'第 2 页'})).toBeVisible();expect(screen.getByRole('figure',{name:'第 3 页'})).toBeVisible();
  screen.getByRole('main',{name:'页面预览'}).focus();await user.keyboard('{PageDown}');expect(choose).toHaveValue('page-4');
  await user.click(within(screen.getByRole('figure',{name:'第 4 页'})).getByRole('button',{name:'选择照片框 1'}));
  const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'120');
  expect(screen.getByRole('button',{name:'返回页面概览'})).toBeDisabled();expect(screen.getByRole('button',{name:'下一跨页'})).toBeDisabled();
  screen.getByRole('main',{name:'页面预览'}).focus();await user.keyboard('{PageDown}');expect(choose).toHaveValue('page-4');
  await user.click(screen.getByRole('button',{name:'放弃调整'}));await user.click(screen.getByRole('button',{name:'返回页面概览'}));
  expect(screen.getByRole('region',{name:'页面概览'})).toBeVisible();
  expect(repository.editFrame).not.toHaveBeenCalled();
});

it('空页概览提供真实的文字和尺寸入口，历史只浏览而不显示写入入口',async()=>{
  const empty=structuredClone(doc);empty.layout.pages[0].frames=[];
  const user=userEvent.setup();setup({document:vi.fn().mockResolvedValue(empty),addText:vi.fn(),editPage:vi.fn(),version:vi.fn().mockResolvedValue({version,layout:empty.layout,previews:[]})});
  const overview=await screen.findByRole('region',{name:'页面概览'});
  expect(overview).toHaveTextContent('190 × 257 mm');
  await user.click(within(overview).getByRole('button',{name:'添加文字内容'}));
  expect(screen.getByRole('dialog',{name:'添加文字'})).toBeVisible();
  await user.click(within(screen.getByRole('dialog',{name:'添加文字'})).getByRole('button',{name:'关闭对话框'}));
  await user.click(within(overview).getByRole('button',{name:'调整这页尺寸'}));
  expect(screen.getByRole('dialog',{name:'页面设置'})).toBeVisible();
  await user.click(within(screen.getByRole('dialog',{name:'页面设置'})).getByRole('button',{name:'关闭对话框'}));
  await user.click(screen.getByRole('tab',{name:'版本记录'}));await user.click(screen.getByRole('button',{name:/^确认版/}));
  expect(screen.getByRole('region',{name:'页面概览'})).toBeVisible();
  expect(screen.queryByRole('button',{name:'添加文字内容'})).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:'调整这页尺寸'})).not.toBeInTheDocument();
});

it('打开章节管理清除上一条保存提示，不把已保存状态带入新的草稿窗口',async()=>{
  const {continuousFixture}=await import('../../test/publicationContinuousFixture'),{layout}=continuousFixture();layout.publication.id='book';layout.publication.projectId='p';
  setup({document:vi.fn().mockResolvedValue({revision:2,layout,previews:[]}),setChapters:vi.fn()});const user=userEvent.setup();
  await user.type(await screen.findByRole('textbox',{name:'版本名称'}),'章节前');await user.click(screen.getByRole('button',{name:'保存版本'}));
  await waitFor(()=>expect(screen.getByRole('status',{name:'版面状态'})).toHaveTextContent('版本已保存。'));
  await user.click(screen.getByRole('tab',{name:'页面'}));await user.click(screen.getByRole('button',{name:'管理章节'}));
  expect(within(screen.getByRole('dialog',{name:'管理章节'})).queryByText('版本已保存。')).not.toBeInTheDocument();
});
