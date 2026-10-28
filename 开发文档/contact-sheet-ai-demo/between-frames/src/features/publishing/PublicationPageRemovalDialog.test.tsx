import { cleanup,render,screen,within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { setCopyLocale,zh } from '../../app/copy';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationChaptersPreview } from './publicationChapterPlans';
import { PublicationPageRemovalDialog } from './PublicationPageRemovalDialog';
beforeEach(()=>setCopyLocale('zh'));afterEach(()=>{cleanup();setCopyLocale('zh');});
function scope(){const {layout}=continuousFixture(),chapter={id:crypto.randomUUID(),title:'夜行',startPageId:layout.pages[1].id};return {document:{revision:4,layout:publicationChaptersPreview(layout,{chapters:[chapter],pageIds:layout.pages.map(p=>p.id)}),previews:[]},chapterId:chapter.id,pageIds:layout.pages.slice(1).map(p=>p.id)};}
it('先预览具体页面数量，取消不写，保留空白选择仅在确认后随冻结范围提交',async()=>{
 const current=scope(),onConfirm=vi.fn().mockResolvedValue(true),onClose=vi.fn(),user=userEvent.setup();
 render(<PublicationPageRemovalDialog scope={current} busy={false} pending={false} stale={false} notice="" onClose={onClose} onConfirm={onConfirm}/>);
 expect(screen.getByText('涉及 3 页、3 个照片框、0 处文字。')).toBeVisible();
 await user.click(screen.getByRole('radio',{name:/^保留第一页为空白/}));
 expect(onConfirm).not.toHaveBeenCalled();expect(screen.getByText('将移除 2 页，保留 1 页空白。')).toBeVisible();
 await user.selectOptions(screen.getByRole('combobox',{name:'查看待移除页面'}),current.pageIds[1]);expect(onConfirm).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button',{name:'确认移除页面与内容'}));
 expect(onConfirm).toHaveBeenCalledExactlyOnceWith({pageIds:current.pageIds,chapterId:current.chapterId,retainBlankPageId:current.pageIds[0]},current.pageIds[0]);expect(onClose).toHaveBeenCalledOnce();
});
it('未知回执和已更新版面冻结范围选项并阻止重复提交，返回可离开',async()=>{
 const current=scope(),onConfirm=vi.fn(),onClose=vi.fn(),user=userEvent.setup(),props={scope:current,busy:false,pending:true,stale:false,notice:'等待确认',onClose,onConfirm};
 const view=render(<PublicationPageRemovalDialog {...props}/>);expect(screen.getByRole('button',{name:'确认移除页面与内容'})).toBeDisabled();for(const option of screen.getAllByRole('radio'))expect(option).toBeDisabled();
 view.rerender(<PublicationPageRemovalDialog {...props} pending={false} stale/>);expect(screen.getByRole('alert')).toHaveTextContent('版面已有更新');expect(screen.getByRole('button',{name:'确认移除页面与内容'})).toBeDisabled();
 await user.click(screen.getByRole('button',{name:'返回'}));expect(onClose).toHaveBeenCalledOnce();expect(onConfirm).not.toHaveBeenCalled();
});
it('四语预览保留原图说明，返回与关闭不会执行移除',async()=>{
 const current=scope();for(const locale of ['zh','en','ja','ko']){
  setCopyLocale(locale);const onConfirm=vi.fn(),onClose=vi.fn(),user=userEvent.setup();
  render(<PublicationPageRemovalDialog scope={current} busy={false} pending={false} stale={false} notice="" onClose={onClose} onConfirm={onConfirm}/>);
  expect(within(screen.getByRole('dialog')).getByText(zh.publicationRemovePagesHint)).toBeVisible();await user.click(screen.getByRole('button',{name:zh.publicationBack}));expect(onClose).toHaveBeenCalledOnce();expect(onConfirm).not.toHaveBeenCalled();cleanup();
 }
});

it('所选页面的隐藏锁定内容不能通过选择保留空白来绕过',async()=>{
 const current=scope(),{publicationLayers,publicationLayerPreview}=await import('../../domain/publicationLayers'),tree=publicationLayers(current.document.layout),frame=current.document.layout.pages[1].frames[0];
 tree.nodes.find(n=>n.id===frame.id)!.locked=true;tree.nodes.find(n=>n.id===frame.id)!.visible=false;current.document.layout=publicationLayerPreview(current.document.layout,tree);
 const onConfirm=vi.fn();render(<PublicationPageRemovalDialog scope={current} busy={false} pending={false} stale={false} notice="" onClose={vi.fn()} onConfirm={onConfirm}/>);
 expect(screen.getByRole('button',{name:'确认移除页面与内容'})).toBeDisabled();for(const option of screen.getAllByRole('radio'))expect(option).toBeDisabled();expect(screen.getAllByText(/所选页面包含锁定内容/).length).toBeGreaterThan(0);expect(onConfirm).not.toHaveBeenCalled();
});
