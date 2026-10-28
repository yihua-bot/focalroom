import { useState } from 'react';
import { cleanup,fireEvent,render,screen } from '@testing-library/react';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { usePublicationTransform } from './usePublicationTransform';
import type { PublicationObject,PublicationRectangle } from './publicationGeometry';
import type { PublicationObjectGeometry } from './publicationArrangement';
const original={xMm:15,yMm:20,widthMm:80,heightMm:100};
function Surface({change,readOnly=false,only}:{change:(box:PublicationRectangle)=>void;readOnly?:boolean;only?:PublicationObject}){
  const [box,setBox]=useState(original);const handlers=usePublicationTransform({page:{widthMm:200,heightMm:300,bleedMm:3},objects:[{kind:'frame',id:'photo',...box}],only,onSelect:vi.fn(),onChange:readOnly?undefined:(_object,value)=>{change(value);setBox(value);}});
  return <div data-testid="paper" {...handlers}><button data-publication-kind="frame" data-publication-id="photo">照片</button><button data-publication-kind="frame" data-publication-id="photo" data-publication-corner="se">角点</button><input aria-label="备注"/></div>;
}
beforeEach(()=>{vi.stubGlobal('PointerEvent',class extends MouseEvent {pointerId:number;isPrimary:boolean;constructor(type:string,init:PointerEventInit={}){super(type,init);this.pointerId=init.pointerId??1;this.isPrimary=init.isPrimary??true;}});vi.spyOn(HTMLElement.prototype,'getBoundingClientRect').mockReturnValue({x:0,y:0,left:0,top:0,right:200,bottom:300,width:200,height:300,toJSON:()=>({})});});
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.unstubAllGlobals();});
it('小位移不开始拖动，移动在外部窗口继续，松手不自动保存或再改位置',()=>{
  const change=vi.fn();render(<Surface change={change}/>);fireEvent.pointerDown(screen.getByRole('button',{name:'照片'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:21,clientY:31});expect(change).not.toHaveBeenCalled();fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:60});expect(change).toHaveBeenLastCalledWith({...original,xMm:35,yMm:50});fireEvent.pointerUp(window,{pointerId:1});const count=change.mock.calls.length;fireEvent.pointerMove(window,{pointerId:1,clientX:100,clientY:100});expect(change).toHaveBeenCalledTimes(count);
});
it('Esc、指针取消和窗口尺寸变化恢复拖动前的预览',()=>{
  for(const cancel of [()=>fireEvent.keyDown(window,{key:'Escape'}),()=>fireEvent.pointerCancel(window,{pointerId:1}),()=>fireEvent(window,new Event('resize'))]){const change=vi.fn();render(<Surface change={change}/>);fireEvent.pointerDown(screen.getByRole('button',{name:'照片'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:60});cancel();expect(change).toHaveBeenLastCalledWith(original);cleanup();}
});
it('键盘微调及角点调整有毫米语义，不劫持输入框或只读界面',()=>{
  const change=vi.fn();const view=render(<Surface change={change}/>);fireEvent.keyDown(screen.getByRole('button',{name:'照片'}),{key:'ArrowRight'});expect(change).toHaveBeenLastCalledWith({...original,xMm:16});fireEvent.keyDown(screen.getByRole('button',{name:'照片'}),{key:'ArrowDown',shiftKey:true});expect(change).toHaveBeenLastCalledWith({...original,xMm:16,yMm:30});fireEvent.keyDown(screen.getByRole('button',{name:'角点'}),{key:'ArrowLeft'});expect(change).toHaveBeenLastCalledWith({...original,xMm:16,yMm:30,widthMm:79});const count=change.mock.calls.length;fireEvent.keyDown(screen.getByRole('textbox',{name:'备注'}),{key:'ArrowRight'});expect(change).toHaveBeenCalledTimes(count);view.rerender(<Surface change={change} readOnly/>);fireEvent.keyDown(screen.getByRole('button',{name:'照片'}),{key:'ArrowRight'});expect(change).toHaveBeenCalledTimes(count);
});
it('未保存其他对象时不可开始新拖动，卸载只清理监听而不回写父界面',()=>{
  const change=vi.fn();const view=render(<Surface change={change} only={{kind:'text',id:'other'}}/>);fireEvent.pointerDown(screen.getByRole('button',{name:'照片'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:60});expect(change).not.toHaveBeenCalled();view.rerender(<Surface change={change}/>);fireEvent.pointerDown(screen.getByRole('button',{name:'照片'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:60});const count=change.mock.calls.length;view.unmount();fireEvent.pointerMove(window,{pointerId:1,clientX:100,clientY:100});expect(change).toHaveBeenCalledTimes(count);
});

function GroupSurface({change,onAll}:{change:(items:PublicationObjectGeometry[])=>void;onAll:()=>void}){
  const [objects,setObjects]=useState<PublicationObjectGeometry[]>([{kind:'frame',id:'photo',xMm:15,yMm:20,widthMm:40,heightMm:50},{kind:'text',id:'caption',xMm:90,yMm:80,widthMm:30,heightMm:20}]);
  const handlers=usePublicationTransform({page:{widthMm:200,heightMm:300,bleedMm:3},objects,selection:objects,selectionId:'page',only:{kind:'selection',id:'page'},onSelect:vi.fn(),onSelectAll:onAll,onChange:(_object,_box,members)=>{if(members){change(members);setObjects(members);}}});
  return <div {...handlers}><button data-publication-kind="frame" data-publication-id="photo">组选中照片</button><button data-publication-kind="selection" data-publication-id="page" data-publication-corner="se">组选区角点</button><input aria-label="组内备注"/></div>;
}
it('拖动组选中成员使用冻结起点，连续预览和Esc不会累计位置漂移',()=>{
  const change=vi.fn();render(<GroupSurface change={change} onAll={vi.fn()}/>);fireEvent.pointerDown(screen.getByRole('button',{name:'组选中照片'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:60});expect(change.mock.calls[0][0].map((o:PublicationObjectGeometry)=>o.xMm)).toEqual([35,110]);fireEvent.pointerMove(window,{pointerId:1,clientX:60,clientY:70});expect(change.mock.calls[1][0].map((o:PublicationObjectGeometry)=>o.xMm)).toEqual([55,130]);fireEvent.keyDown(window,{key:'Escape'});expect(change.mock.calls[2][0].map((o:PublicationObjectGeometry)=>[o.xMm,o.yMm])).toEqual([[15,20],[90,80]]);
});
it('组选区角点与键盘操作全部成员，选择全部不接管输入框',()=>{
  const change=vi.fn(),all=vi.fn();render(<GroupSurface change={change} onAll={all}/>);fireEvent.keyDown(screen.getByRole('button',{name:'组选区角点'}),{key:'ArrowRight'});expect(change.mock.calls[0][0]).toHaveLength(2);expect(change.mock.calls[0][0][0].widthMm).toBeGreaterThan(40);fireEvent.keyDown(screen.getByRole('button',{name:'组选中照片'}),{key:'a',metaKey:true});expect(all).toHaveBeenCalledOnce();fireEvent.keyDown(screen.getByRole('textbox',{name:'组内备注'}),{key:'a',metaKey:true});expect(all).toHaveBeenCalledOnce();
});

it('跨页边界与单页的屏幕比例分开，右侧拖动20像素只移动20毫米而不翻倍',()=>{
  const change=vi.fn();function SpreadSurface(){const handlers=usePublicationTransform({page:{widthMm:200,heightMm:300,bleedMm:3},boundsFor:()=>({widthMm:400,heightMm:300,bleedMm:3}),objects:[{kind:'frame',id:'cross',xMm:20,yMm:20,widthMm:350,heightMm:240}],onSelect:vi.fn(),onChange:(_object,box)=>change(box)});return <div {...handlers}><button data-publication-kind="frame" data-publication-id="cross">右侧视窗</button></div>;}
  render(<SpreadSurface/>);fireEvent.pointerDown(screen.getByRole('button',{name:'右侧视窗'}),{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:40,clientY:40});expect(change).toHaveBeenLastCalledWith({xMm:40,yMm:30,widthMm:350,heightMm:240});fireEvent.pointerUp(window,{pointerId:1});fireEvent.keyDown(screen.getByRole('button',{name:'右侧视窗'}),{key:'ArrowRight'});expect(change).toHaveBeenLastCalledWith({xMm:21,yMm:20,widthMm:350,heightMm:240});
});

it('吸附只处理指针预览，Alt绕过并清除参考，Esc还原；方向键不被吸附吞掉',()=>{
 const change=vi.fn(),adjust=vi.fn((box:PublicationRectangle)=>({...box,xMm:25})),clear=vi.fn();
 function SnapSurface(){const [box,setBox]=useState(original);const handlers=usePublicationTransform({page:{widthMm:200,heightMm:300,bleedMm:3},objects:[{kind:'frame',id:'photo',...box}],pointerTransform:adjust,onPointerFeedbackClear:clear,onSelect:vi.fn(),onChange:(_o,value)=>{change(value);setBox(value);}});return <div {...handlers}><button data-publication-kind="frame" data-publication-id="photo">吸附照片</button></div>;}
 render(<SnapSurface/>);const photo=screen.getByRole('button',{name:'吸附照片'});fireEvent.pointerDown(photo,{pointerId:1,button:0,clientX:20,clientY:30});fireEvent.pointerMove(window,{pointerId:1,clientX:28.4,clientY:30});expect(change).toHaveBeenLastCalledWith({...original,xMm:25});fireEvent.pointerMove(window,{pointerId:1,clientX:28.4,clientY:30,altKey:true});expect(change).toHaveBeenLastCalledWith({...original,xMm:23.4});expect(adjust).toHaveBeenCalledOnce();expect(clear).toHaveBeenCalledOnce();fireEvent.keyDown(window,{key:'Escape'});expect(change).toHaveBeenLastCalledWith(original);fireEvent.keyDown(photo,{key:'ArrowRight'});expect(change).toHaveBeenLastCalledWith({...original,xMm:16});expect(adjust).toHaveBeenCalledOnce();
});
