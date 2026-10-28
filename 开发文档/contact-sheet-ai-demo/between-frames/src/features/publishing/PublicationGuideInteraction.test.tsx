import { cleanup,fireEvent,render,screen } from '@testing-library/react';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { PublicationPaper } from './PublicationPaper';
import { publicationContinuousFrames,publicationLinkPreview } from '../../domain/publicationContinuous';
import { publicationGuidesPreview,publicationPageGuides,publicationDefaultGuideView } from './publicationGuides';
beforeEach(()=>{vi.stubGlobal('PointerEvent',class extends MouseEvent {pointerId:number;isPrimary=true;constructor(type:string,init:PointerEventInit={}){super(type,init);this.pointerId=init.pointerId??1;}});vi.spyOn(HTMLElement.prototype,'getBoundingClientRect').mockReturnValue({x:0,y:0,left:0,top:0,right:190,bottom:257,width:190,height:257,toJSON:()=>({})});});
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.unstubAllGlobals();});
it('整组拖动按组选区吸附并排除自身成员，不改变组内距离、裁切或原对象',()=>{
 const {layout}=continuousFixture(),page=layout.pages[0];page.frames[0].widthMm=40;page.frames[0].heightMm=50;const text={id:crypto.randomUUID(),xMm:65,yMm:20,widthMm:20,heightMm:20,contentJson:'{"kind":"text","text":"组内文字"}',styleJson:null};page.texts=[text];page.layoutJson=publicationGuidesPreview(page,{...publicationPageGuides(page),verticalGuidesMm:[25],horizontalGuidesMm:[20]});const original=structuredClone(page),change=vi.fn();render(<PublicationPaper page={page} previews={[]} label="版面" onSelect={vi.fn()} onSelectText={vi.fn()} onTransform={change} selectedObjects={[{kind:'frame',id:page.frames[0].id},{kind:'text',id:text.id}]} assist={{...publicationDefaultGuideView,snap:true}}/>);
 fireEvent.pointerDown(screen.getByRole('button',{name:'选择照片框 1'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:29.5,clientY:30});expect(change).toHaveBeenCalledOnce();const [object,box,members]=change.mock.calls[0];expect(object.kind).toBe('selection');expect(box.xMm).toBe(25);expect(members.map((member:{xMm:number})=>member.xMm)).toEqual([25,75]);expect(page).toEqual(original);fireEvent.pointerUp(window,{pointerId:1});expect(document.querySelectorAll('.publication-guide-snap line')).toHaveLength(0);
});
it('右页拖动连续照片按完整跨页坐标吸附，左右共用来源不会翻倍位移',()=>{
 const {layout,data}=continuousFixture();layout.pages[2].layoutJson=publicationGuidesPreview(layout.pages[2],{...publicationPageGuides(layout.pages[2]),verticalGuidesMm:[25]});const linked=publicationLinkPreview(layout,data),items=publicationContinuousFrames(linked),change=vi.fn(),before=structuredClone(linked);render(<PublicationPaper page={items[0].right} continuous={items} previews={[]} label="右页" onSelect={vi.fn()} onTransform={change} assist={{...publicationDefaultGuideView,snap:true}}/>);
 fireEvent.pointerDown(screen.getByRole('button',{name:'跨页照片 1'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:29.5,clientY:30});expect(change.mock.calls[0][1].xMm).toBe(25);expect(change.mock.calls[0][1].widthMm).toBe(data.geometry.widthMm);expect(linked).toEqual(before);fireEvent.pointerUp(window,{pointerId:1});
});
