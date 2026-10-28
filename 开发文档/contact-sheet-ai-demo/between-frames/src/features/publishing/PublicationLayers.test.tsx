import { cleanup,fireEvent,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationLayers,publicationLayerViews,publicationChangeLayers } from '../../domain/publicationLayers';
import { PublicationLayersPanel } from './PublicationLayers';
import { PublicationPaper } from './PublicationPaper';
import { setCopyLocale } from '../../app/copy';
afterEach(()=>{cleanup();setCopyLocale('zh');});
it('图层覆盖图文顺序，隐藏不创建图像，锁定仍可选择但不出现调整角点或键盘写入',()=>{
 const {layout}=continuousFixture(),page=layout.pages[0],frame=page.frames[0],change=vi.fn(),layers=publicationLayers(layout);let next=publicationChangeLayers(layout,layers,{type:'locked',id:frame.id});
 const view=render(<PublicationPaper page={page} previews={[{frameId:frame.id,thumbnailSrc:'/photo.jpg'}]} layers={publicationLayerViews(next)} selectedFrame={frame.id} label="当前" onSelect={vi.fn()} onTransform={change}/>);
 expect(screen.getByRole('button',{name:'选择照片框 1'})).toBeEnabled();expect(document.querySelectorAll('.publication-resize-handle')).toHaveLength(0);fireEvent.keyDown(screen.getByRole('button',{name:'选择照片框 1'}),{key:'ArrowRight'});expect(change).not.toHaveBeenCalled();
 next=publicationChangeLayers(layout,next,{type:'visible',id:frame.id});view.rerender(<PublicationPaper page={page} previews={[{frameId:frame.id,thumbnailSrc:'/photo.jpg'}]} layers={publicationLayerViews(next)} selectedFrame={frame.id} label="当前" onSelect={vi.fn()} onTransform={change}/>);expect(document.querySelectorAll('img')).toHaveLength(0);
});
it('四语图层列表有明确显示和锁定操作，不暴露翻译键；历史视图不能保存',async()=>{
 for(const locale of ['zh','en','ja','ko']){
  setCopyLocale(locale);const {layout}=continuousFixture(),layers=publicationLayers(layout),action=vi.fn(),save=vi.fn(),user=userEvent.setup();
  render(<PublicationLayersPanel layers={layers} sceneIds={[layout.pages[0].frames[0].id]} selectedIds={[]} activeId={layout.pages[0].frames[0].id} disabled dirty canGroup={false} label={()=>'A'} onSelect={vi.fn()} onAction={action} onSave={save} onDiscard={vi.fn()}/>);
  expect(document.body.textContent).not.toMatch(/publication[A-Z]/);expect(screen.getByRole('button',{name:locale==='zh'?'保存图层':locale==='en'?'Save layers':locale==='ja'?'レイヤーを保存':'레이어 저장'})).toBeDisabled();await user.keyboard('{Enter}');expect(action).not.toHaveBeenCalled();expect(save).not.toHaveBeenCalled();cleanup();
 }
});

it('多选包含隐藏成员时不退化成只移动可见成员，必须先显示或重新选择',()=>{
 const {layout}=continuousFixture(),page=layout.pages[0],frame=page.frames[0];const text={id:crypto.randomUUID(),xMm:20,yMm:20,widthMm:50,heightMm:20,contentJson:'{"kind":"text","text":"隐藏成员"}',styleJson:null};page.texts=[text];
 const data=publicationChangeLayers(layout,publicationLayers(layout),{type:'visible',id:text.id}),change=vi.fn();render(<PublicationPaper page={page} previews={[]} selectedObjects={[{kind:'frame',id:frame.id},{kind:'text',id:text.id}]} layers={publicationLayerViews(data)} label="当前" onSelect={vi.fn()} onTransform={change}/>);
 fireEvent.keyDown(screen.getByRole('button',{name:'选择照片框 1'}),{key:'ArrowRight'});expect(change).not.toHaveBeenCalled();expect(document.querySelectorAll('.publication-resize-handle')).toHaveLength(0);
});

it('组内任一成员锁定会阻止整组移除，并解释原因；解除分组仍是独立操作',async()=>{
 const {layout}=continuousFixture(),page=layout.pages[0],text={id:crypto.randomUUID(),xMm:20,yMm:20,widthMm:50,heightMm:20,contentJson:'{"kind":"text","text":"锁定成员"}',styleJson:null};page.texts=[text];const group=crypto.randomUUID();let data=publicationChangeLayers(layout,publicationLayers(layout),{type:'group',id:group,ids:[page.frames[0].id,text.id],name:'封面'});
 data=publicationChangeLayers(layout,data,{type:'locked',id:text.id});const remove=vi.fn(),action=vi.fn(),user=userEvent.setup();render(<PublicationLayersPanel layers={data} sceneIds={[page.frames[0].id,text.id]} selectedIds={[]} activeId={group} disabled={false} dirty={false} canGroup={false} label={node=>node.name??node.kind} onSelect={vi.fn()} onAction={action} onSave={vi.fn()} onDiscard={vi.fn()} onRemove={remove}/>);
 expect(screen.getByRole('button',{name:'移除整组内容…'})).toBeDisabled();expect(screen.getByText('组内有已锁定的内容，请先解锁后再移除。')).toBeVisible();await user.click(screen.getByRole('button',{name:'解除分组'}));expect(action).toHaveBeenCalledExactlyOnceWith('ungroup',group);expect(remove).not.toHaveBeenCalled();
});
