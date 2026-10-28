import { cleanup,render,screen,within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationContinuousFrames,publicationLinkPreview } from '../../domain/publicationContinuous';
import { PublicationPaper } from './PublicationPaper';
import { PublicationContinuousDialog } from './PublicationContinuousDialog';
import { setCopyLocale,zh } from '../../app/copy';
afterEach(()=>{cleanup();setCopyLocale('zh');});
it('两个视窗使用同一来源与完整照片尺寸，仅生成四个外侧调整角点',()=>{
  const {layout,data}=continuousFixture(),preview=publicationLinkPreview(layout,data),items=publicationContinuousFrames(preview),previews=[{frameId:data.frameId,thumbnailSrc:'/one-photo.jpg'}],select=vi.fn();
  const {container}=render(<>{[items[0].left,items[0].right].map(page=><PublicationPaper key={page.id} page={{...page,frames:page.frames.filter(frame=>frame.id===data.frameId)}} continuous={items} previews={previews} selectedFrame={data.frameId} onSelect={select} onTransform={vi.fn()} label="版面"/>)}</>);
  const halves=screen.getAllByRole('button',{name:'跨页照片 1'});expect(halves).toHaveLength(2);expect(halves[0].querySelector('img')?.src).toBe(halves[1].querySelector('img')?.src);expect(halves[0].querySelector('.publication-continuous-image')).toHaveStyle({width:'200%',left:'0%'});expect(halves[1].querySelector('.publication-continuous-image')).toHaveStyle({width:'200%',left:'-100%'});expect(container.querySelectorAll('.publication-resize-handle')).toHaveLength(4);
});
it('缺少跨页来源预览时每侧都明确提示，不拿其他照片替代',()=>{
  const {layout,data}=continuousFixture(),items=publicationContinuousFrames(publicationLinkPreview(layout,data));render(<>{[items[0].left,items[0].right].map(page=><PublicationPaper key={page.id} page={{...page,frames:page.frames.filter(frame=>frame.id===data.frameId)}} continuous={items} previews={[]} label="版面"/>)}</>);expect(screen.getAllByText('此图像版本的预览暂不可用')).toHaveLength(2);expect(document.querySelectorAll('img')).toHaveLength(0);
});
it('跨页展开是明确预览与确认，不自动保存；不跨装订线时禁止确认',async()=>{
  const {layout,data}=continuousFixture(),user=userEvent.setup(),save=vi.fn().mockResolvedValue(true),close=vi.fn();render(<PublicationContinuousDialog editor={{mode:'link',layout,previews:[],frameId:data.frameId,leftId:data.leftPageId,rightId:data.rightPageId,revision:8}} busy={false} pending={false} canWrite notice="" onClose={close} onLink={save} onUnlink={vi.fn()}/>);
  expect(save).not.toHaveBeenCalled();expect(screen.getAllByRole('button',{name:'跨页照片 1'})).toHaveLength(2);
  const width=screen.getByRole('spinbutton',{name:'宽度（mm）'});await user.clear(width);await user.type(width,'100');expect(screen.getByRole('alert')).toBeVisible();expect(screen.getByRole('button',{name:'展开为跨页照片'})).toBeDisabled();
  await user.clear(width);await user.type(width,'350');await user.click(screen.getByRole('button',{name:'展开为跨页照片'}));expect(save).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({frameId:data.frameId,leftPageId:data.leftPageId,rightPageId:data.rightPageId,geometry:expect.objectContaining({widthMm:350,cropX:.2,cropY:.7,cropZoom:1.6})}));expect(close).toHaveBeenCalledOnce();
});
it('转单页必须明确保留页面，保存结果不明或只读时不能重复提交；四语言不暴露翻译键',async()=>{
  for(const locale of ['zh','en','ja','ko']){
    setCopyLocale(locale);const {layout,data}=continuousFixture(),user=userEvent.setup(),save=vi.fn(),close=vi.fn();render(<PublicationContinuousDialog editor={{mode:'unlink',layout:publicationLinkPreview(layout,data),previews:[],frameId:data.frameId,leftId:data.leftPageId,rightId:data.rightPageId,revision:9}} busy={false} pending canWrite notice="" onClose={close} onLink={vi.fn()} onUnlink={save}/>);
    expect(document.body.textContent).not.toMatch(/publication[A-Z]/);const dialog=screen.getByRole('dialog');expect(within(dialog).getAllByRole('button').filter(button=>button.getAttribute('type')==='submit')[0]).toBeDisabled();expect(save).not.toHaveBeenCalled();await user.click(within(dialog).getByRole('button',{name:zh.close}));expect(close).toHaveBeenCalledOnce();cleanup();
  }
});
