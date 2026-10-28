import { cleanup,render,screen,within,waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { setCopyLocale,zh } from '../../app/copy';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { PublicationChaptersDialog,PublicationChapterOpeningDialog } from './PublicationChapterDialogs';
import { publicationChaptersPreview } from './publicationChapterPlans';
import type { PublicationDocument } from '../../domain/publication';
beforeEach(()=>setCopyLocale('zh'));afterEach(()=>{cleanup();setCopyLocale('zh');});
function fixture():PublicationDocument{const {layout}=continuousFixture();return {revision:3,layout,previews:[]};}
it('标记章节只修改预览，关闭需确认，保存才交付起始页；移除标记保留原页面',async()=>{
 const document=fixture(),before=structuredClone(document),onSave=vi.fn().mockResolvedValue(true),onClose=vi.fn(),user=userEvent.setup();
 render(<PublicationChaptersDialog document={document} busy={false} pending={false} notice="" onClose={onClose} onSave={onSave}/>);
 await user.click(screen.getByRole('button',{name:'从现有页面开始一章'}));
 await user.clear(screen.getByRole('textbox',{name:'章节名称'}));await user.type(screen.getByRole('textbox',{name:'章节名称'}),'第一章');
 expect(onSave).not.toHaveBeenCalled();expect(document).toEqual(before);
 await user.click(screen.getByRole('button',{name:'关闭对话框'}));expect(screen.getByRole('group',{name:'放弃尚未保存的调整？'})).toBeVisible();expect(onClose).not.toHaveBeenCalled();
 await user.click(within(screen.getByRole('group',{name:'放弃尚未保存的调整？'})).getByRole('button',{name:'返回'}));
 await user.click(screen.getByRole('button',{name:'保存章节调整'}));
 expect(onSave).toHaveBeenCalledWith({chapters:[expect.objectContaining({title:'第一章',startPageId:document.layout.pages[1].id})],pageIds:document.layout.pages.map(p=>p.id)});
 expect(onClose).toHaveBeenCalledOnce();
 cleanup();
 const chapter={id:crypto.randomUUID(),title:'第一章',startPageId:document.layout.pages[1].id};
 const saved={...document,layout:publicationChaptersPreview(document.layout,{chapters:[chapter],pageIds:document.layout.pages.map(p=>p.id)})};
 render(<PublicationChaptersDialog document={saved} busy={false} pending={false} notice="" onClose={vi.fn()} onSave={onSave}/>);
 await user.click(screen.getByRole('button',{name:'移除章节标记 第一章'}));expect(document).toEqual(before);
 await user.click(screen.getByRole('button',{name:'保存章节调整'}));expect(onSave).toHaveBeenLastCalledWith({chapters:[],pageIds:document.layout.pages.map(p=>p.id)});
});
it('新建章节先预览真实标题页，非法右页组合阻止提交，待确认时冻结全部请求选项',async()=>{
 const document=fixture(),onSave=vi.fn().mockResolvedValue(false),user=userEvent.setup(),props={document,afterPageId:document.layout.pages[1].id,busy:false,pending:false,notice:'',onClose:vi.fn(),onSave};
 const view=render(<PublicationChapterOpeningDialog {...props}/>);
 await user.type(screen.getByRole('textbox',{name:'章节名称'}),'第三章');
 expect(screen.getByRole('figure',{name:'第 3 页'})).toHaveTextContent('第三章');
 expect(onSave).not.toHaveBeenCalled();
 await user.selectOptions(screen.getByRole('combobox',{name:'新增页面'}),'1');expect(screen.getByRole('button',{name:'创建章节与页面'})).toBeDisabled();expect(screen.getByRole('alert')).toHaveTextContent('此位置无法');
 await user.selectOptions(screen.getByRole('combobox',{name:'新增页面'}),'2');await user.click(screen.getByRole('button',{name:'创建章节与页面'}));expect(onSave).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({chapter:expect.objectContaining({title:'第三章'}),pages:expect.any(Array),startOnRight:true}));
 view.rerender(<PublicationChapterOpeningDialog {...props} pending/>);
 expect(screen.getByRole('textbox',{name:'章节名称'})).toBeDisabled();expect(screen.getByRole('combobox',{name:'新增页面'})).toBeDisabled();expect(screen.getByRole('button',{name:'创建章节与页面'})).toBeDisabled();
});
it('章节创建预览覆盖四语，不能把未命名占位当作已填写名称提交',async()=>{
 const document=fixture();for(const locale of ['zh','en','ja','ko']){
  setCopyLocale(locale);const onSave=vi.fn();render(<PublicationChapterOpeningDialog document={document} afterPageId={document.layout.pages[1].id} busy={false} pending={false} notice="" onClose={vi.fn()} onSave={onSave}/>);
  expect(screen.getByRole('button',{name:zh.publicationCreateChapterConfirm})).toBeDisabled();expect(screen.getByRole('textbox',{name:zh.publicationChapterName})).toHaveValue('');
  await waitFor(()=>expect(screen.getByRole('figure',{name:zh.publicationPageNumber.replace('{number}','3')})).toBeVisible());expect(onSave).not.toHaveBeenCalled();expect(globalThis.document.body.textContent).not.toMatch(/publication[A-Z]/);cleanup();
 }
});
