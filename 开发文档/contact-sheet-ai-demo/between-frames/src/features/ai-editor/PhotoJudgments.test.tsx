import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { PhotoJudgments } from './PhotoJudgments';
import { photoJudgmentSchema } from '../../services/photoJudgmentSchemas';
import type { PhotoJudgment } from '../../domain/photoJudgment';

const item:PhotoJudgment={id:'judgment',photoId:'a',observationCallId:'observe',judgment:'主体可能不够突出',reasoning:'明亮背景可能竞争注意力',facts:[{index:0,text:'背景明亮'}],uncertainty:'需结合拍摄意图',observationUncertainty:'表情看不清',imageStatus:'verified',inference:true,methods:[{callId:'method',citationId:'FR-1',title:'主体与环境',passage:'比较主体和背景的关系',kind:'original_method',reviewStatus:'original',disputed:false,excerpt:false,source:'focalroom://method'}]};
describe('单张判断与依据',()=>{
 it('展开后分别呈现推理、原始观察、方法来路和限制',async()=>{
  render(<PhotoJudgments items={[item]} photoName={()=>'a.jpg'} />);
  await userEvent.click(screen.getByText('单张判断与依据'));
  for(const text of ['a.jpg','主体可能不够突出','明亮背景可能竞争注意力','背景明亮','比较主体和背景的关系','需结合拍摄意图','表情看不清']) expect(screen.getByText(text)).toBeVisible();
  expect(screen.getByText('当前预览与观察依据一致')).toBeVisible();
  expect(screen.getByText(/FocalRoom 原创方法札记/)).toBeVisible();
  expect(screen.getByText(/引用来路可核对，不代表判断已被证实/)).toBeVisible();
 });
 it('像素不一致时保留历史依据，无方法时明确空缺',async()=>{
  render(<PhotoJudgments items={[{...item,imageStatus:'mismatch',methods:[]}]} photoName={()=>'a.jpg'} />);
  await userEvent.click(screen.getByText('单张判断与依据'));
  expect(screen.getByText(/保留的是当时的判断依据/)).toBeVisible();
  expect(screen.queryByText('当前预览与观察依据一致')).not.toBeInTheDocument();
  expect(screen.getByText('本条判断没有绑定方法出处。')).toBeVisible();
 });
 it('未核实、有争议和未取得出处分别展示，不变成已核实',async()=>{
  render(<PhotoJudgments items={[{...item,methods:[{...item.methods[0],kind:'retrieved_claim',reviewStatus:'imported',disputed:true,source:null,excerpt:true}]}]} photoName={()=>'a.jpg'} />);
  await userEvent.click(screen.getByText('单张判断与依据'));
  expect(screen.getByText(/已导入，未核实.*有争议/)).toBeVisible();
  expect(screen.getByText(/未取得外部出处/)).toBeVisible();
  expect(screen.getByText(/摘录/)).toBeVisible();
 });
 it('方法出处带上"为什么算这条"，并说清命中不等于证据',async()=>{
  const methods=[{...item.methods[0],matchReason:{kind:'local_terms' as const,termCount:2,matchedTerms:['色彩','调色'],fields:['tags' as const],matched:3,returned:1}}];
  render(<PhotoJudgments items={[{...item,methods}]} photoName={()=>'a.jpg'} />);
  await userEvent.click(screen.getByText('单张判断与依据'));
  expect(screen.getByText('词面命中 2 个词：色彩、调色 · 命中在标签')).toBeVisible();
  expect(screen.getByText('这次一共命中 3 条，只带回前 1 条')).toBeVisible();
  expect(screen.getByText('命中只解释这条怎么被挑出来，不代表它与这张照片有关，也不代表它正确。')).toBeVisible();
  expect(photoJudgmentSchema.parse({...item,methods}).methods[0].matchReason).toEqual(methods[0].matchReason);
 });
 it('IPC 拒绝未知版本状态、没有事实或将解释冒充事实认证的投影',()=>{
  expect(photoJudgmentSchema.parse(item)).toEqual(item);
  for(const bad of [{...item,imageStatus:'guessed'},{...item,facts:[]},{...item,inference:false},{...item,facts:[{index:9,text:'invented'}]}]) expect(()=>photoJudgmentSchema.parse(bad)).toThrow();
 });
});
