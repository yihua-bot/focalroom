import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { setCopyLocale,zh } from '../../app/copy';
import { PublicationCoverDialog } from './PublicationCoverDialog';
afterEach(()=>{cleanup();setCopyLocale('zh');});
function document(){const {layout}=continuousFixture();return {revision:4,layout,previews:[]};}
it('预览尺寸与封面区域不写入，明确保存才提交冻结范围',async()=>{
 setCopyLocale('zh');const doc=document(),save=vi.fn().mockResolvedValue(true),close=vi.fn(),user=userEvent.setup();
 render(<PublicationCoverDialog document={doc} busy={false} pending={false} stale={false} notice="" onSave={save} onClose={close}/>);
 await user.clear(screen.getByRole('spinbutton',{name:'书脊宽度（mm）'}));await user.type(screen.getByRole('spinbutton',{name:'书脊宽度（mm）'}),'10');
 expect(save).not.toHaveBeenCalled();expect(screen.getByText('展开尺寸 390 × 257 mm，出血在页面外另计。')).toBeVisible();
 await user.click(screen.getByRole('button',{name:'保存调整'}));
 expect(save).toHaveBeenCalledExactlyOnceWith({pageId:doc.layout.pages[0].id,backPageId:null,settings:{panelWidthMm:190,spineWidthMm:10},heightMm:257,marginMm:15,bleedMm:3,contentPlacement:'followPanels',clearPageSettingsConfirmed:false});expect(close).toHaveBeenCalledOnce();
});
it('旧页设置必须明确确认才允许转换，取消不保存',async()=>{
 setCopyLocale('zh');const doc=document();doc.layout.pages[0].layoutJson='{"legacy":true}';const save=vi.fn(),close=vi.fn(),user=userEvent.setup();
 render(<PublicationCoverDialog document={doc} busy={false} pending={false} stale={false} notice="" onSave={save} onClose={close}/>);
 expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
 await user.click(screen.getByRole('checkbox',{name:'替换原有整页设置，保留全部照片与文字'}));expect(screen.getByRole('button',{name:'保存调整'})).toBeEnabled();
 await user.click(screen.getByRole('button',{name:'放弃调整'}));expect(close).toHaveBeenCalledOnce();expect(save).not.toHaveBeenCalled();
});
it('未确认回执或旧预览冻结设置与保存，保持可关闭',async()=>{
 setCopyLocale('zh');const props={document:document(),busy:false,pending:true,stale:false,notice:'',onSave:vi.fn(),onClose:vi.fn()};
 const view=render(<PublicationCoverDialog {...props}/>);for(const input of screen.getAllByRole('spinbutton'))expect(input).toBeDisabled();expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
 view.rerender(<PublicationCoverDialog {...props} pending={false} stale/>);expect(screen.getByRole('alert')).toHaveTextContent('版面已有更新');expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
});
it('四语封面设置及关闭不执行保存，不把预览当印刷批准',async()=>{
 for(const locale of ['zh','en','ja','ko']){setCopyLocale(locale);const save=vi.fn(),close=vi.fn(),user=userEvent.setup();
 render(<PublicationCoverDialog document={document()} busy={false} pending={false} stale={false} notice="" onSave={save} onClose={close}/>);
 expect(screen.getByText(zh.publicationCoverSpineHint)).toBeVisible();await user.click(screen.getByRole('button',{name:zh.publicationDiscardChanges}));expect(close).toHaveBeenCalledOnce();expect(save).not.toHaveBeenCalled();cleanup();}
});
