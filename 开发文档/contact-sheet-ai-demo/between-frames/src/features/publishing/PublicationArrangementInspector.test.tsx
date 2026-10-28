import { cleanup,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,beforeEach,expect,it,vi } from 'vitest';
import { setCopyLocale } from '../../app/copy';
import type { PublicationLayout } from '../../domain/publication';
import { PublicationArrangementInspector } from './PublicationArrangementInspector';
const page:PublicationLayout['pages'][number]={id:'page',pageNumber:1,side:'right',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,texts:[],frames:Array.from({length:21},(_,i)=>({id:'f'+i,xMm:5+(i%10)*12,yMm:10+Math.floor(i/10)*20,widthMm:10,heightMm:10,fitMode:'fit',cropX:.5,cropY:.5,cropZoom:1,assetId:null,renditionId:null,source:null}))};
const props={page,selected:[{kind:'frame' as const,id:'f0'},{kind:'frame' as const,id:'f20'}],busy:false,dirty:false,canWrite:true,changes:0,onToggle:vi.fn(),onAll:vi.fn(),onClear:vi.fn(),onArrange:vi.fn(),onSave:vi.fn(),onDiscard:vi.fn()};
beforeEach(()=>{setCopyLocale('zh');vi.clearAllMocks();});afterEach(cleanup);
it('对象列表分页保持选择，参照变化后只预览，未修改不能保存',async()=>{
  render(<PublicationArrangementInspector {...props}/>);const user=userEvent.setup();expect(screen.getAllByRole('checkbox')).toHaveLength(20);expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();await user.click(screen.getByRole('button',{name:'下一页'}));expect(screen.getByRole('checkbox',{name:'选择照片框 21'})).toBeChecked();await user.click(screen.getByRole('checkbox',{name:'选择照片框 21'}));expect(props.onToggle).toHaveBeenCalledExactlyOnceWith({kind:'frame',id:'f20'});await user.selectOptions(screen.getByRole('combobox',{name:'对齐参照'}),'page');await user.click(screen.getByRole('button',{name:'左对齐'}));expect(props.onArrange).toHaveBeenCalledExactlyOnceWith('left','page');expect(props.onSave).not.toHaveBeenCalled();
});
it('未保存时冻结选择，只读不显示可执行的编排，分布至少需要三项',()=>{
  const view=render(<PublicationArrangementInspector {...props} dirty changes={2}/>);expect(screen.getAllByRole('checkbox').every(input=>(input as HTMLInputElement).disabled)).toBe(true);expect(screen.getByRole('button',{name:'横向等距'})).toBeDisabled();expect(screen.getByRole('button',{name:'保存调整'})).toBeEnabled();view.rerender(<PublicationArrangementInspector {...props} canWrite={false}/>);expect(screen.getByRole('button',{name:'左对齐'})).toBeDisabled();expect(screen.getByRole('button',{name:'保存调整'})).toBeDisabled();
});
