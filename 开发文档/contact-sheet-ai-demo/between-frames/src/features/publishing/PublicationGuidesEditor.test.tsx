import { cleanup,render,screen,within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { setCopyLocale,zh } from '../../app/copy';
import { PublicationGuidesEditor } from './PublicationGuidesEditor';
afterEach(()=>{cleanup();setCopyLocale('zh');});
it('网格与参考线预览不会保存；超出版心禁止确认，明确保存携带全部本页设置',async()=>{
 const {layout}=continuousFixture(),page=layout.pages[0],save=vi.fn().mockResolvedValue(true),close=vi.fn(),user=userEvent.setup();render(<PublicationGuidesEditor page={page} layout={layout} previews={[]} busy={false} pending={false} notice="" onClose={close} onSave={save}/>);
 await user.click(screen.getByRole('button',{name:'添加竖向参考线'}));expect(screen.getByRole('spinbutton',{name:'竖向参考线 1（mm）'})).toHaveValue(95);expect(save).not.toHaveBeenCalled();
 const gap=screen.getByRole('spinbutton',{name:'列间距（mm）'});await user.clear(gap);await user.type(gap,'100');expect(screen.getByRole('alert')).toHaveTextContent('网格须放得进页边距内');expect(screen.getByRole('button',{name:'保存本页设置'})).toBeDisabled();await user.clear(gap);await user.type(gap,'5');await user.click(screen.getByRole('button',{name:'保存本页设置'}));expect(save).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({columns:3,rows:3,columnGapMm:5,verticalGuidesMm:[95]}));expect(close).toHaveBeenCalledOnce();expect(page.layoutJson).toBeNull();
});
it('四语参考线编辑有完整名称，待确认时不再保存；返回不写入',async()=>{
 for(const locale of ['zh','en','ja','ko']){setCopyLocale(locale);const {layout}=continuousFixture(),save=vi.fn(),close=vi.fn(),user=userEvent.setup();render(<PublicationGuidesEditor page={layout.pages[0]} layout={layout} previews={[]} busy={false} pending notice="" onClose={close} onSave={save}/>);const dialog=screen.getByRole('dialog',{name:zh.publicationEditGuides});expect(document.body.textContent).not.toMatch(/publication[A-Z]/);expect(within(dialog).getByRole('button',{name:zh.publicationSaveGuides})).toBeDisabled();await user.click(within(dialog).getByRole('button',{name:zh.publicationBack}));expect(save).not.toHaveBeenCalled();expect(close).toHaveBeenCalledOnce();cleanup();}
});

it('旧页面设置过多时明确说明容量原因，不能误导用户反复修改合法行列；四语保留原页',async()=>{
 for(const locale of ['zh','en','ja','ko']){setCopyLocale(locale);const {layout}=continuousFixture(),page=layout.pages[0];page.layoutJson=JSON.stringify({legacy:'汉'.repeat(20000)});const original=page.layoutJson,save=vi.fn(),user=userEvent.setup();render(<PublicationGuidesEditor page={page} layout={layout} previews={[]} busy={false} pending={false} notice="" onClose={vi.fn()} onSave={save}/>);expect(screen.getByRole('alert')).toHaveTextContent(zh.publicationGuidesCapacity);expect(screen.getByRole('alert')).not.toHaveTextContent(zh.publicationGuidesInvalid);await user.click(screen.getByRole('button',{name:zh.publicationSaveGuides}));expect(save).not.toHaveBeenCalled();expect(page.layoutJson).toBe(original);cleanup();}
});
