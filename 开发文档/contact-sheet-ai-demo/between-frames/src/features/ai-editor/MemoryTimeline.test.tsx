import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it } from 'vitest';
import { MemoryTimeline } from './MemoryTimeline';
import type { MemoryTrace,MemoryTraceRead } from '../../domain/memoryTrace';
import { memoryTraceSchema } from '../../services/memoryTraceSchemas';
const read:MemoryTraceRead={memoryId:7,sourceProjectId:'p',sourceProjectTitle:'旅行摄影',origin:'user_confirmed',scope:'project_local',statusAtRead:'confirmed',selectionReason:'confirmed_priority',confirmation:{requestId:'approval-7',createdAt:'2026-11-05T10:00:00Z',action:'ConfirmHypothesis'},memoryTextVersion:'verified',passage:'保留适当留白',currentStatus:'confirmed',disabled:false,applicability:'用于本项目的开篇',evidence:{recorded:2,available:1,excluded:1,missing:0},evidenceRefsTruncated:false};
const trace:MemoryTrace={events:[{id:1,kind:'context_attempt',round:2,createdAt:'2026-11-05T11:00:00Z',callId:null,recorded:true,reads:[read]}],hasEarlierEvents:false};
it('按请求轮次展示来源、检索原因及冻结的确认回执，明确不证明采纳',async()=>{
 render(<MemoryTimeline trace={trace}/>); await userEvent.click(screen.getByText('请求中的记忆来路'));
 expect(screen.getByText(/请求尝试：准备携带.*第 2 轮/)).toBeVisible();
 expect(screen.getByText(/不证明模型收到、采纳或据此作出判断/)).toBeVisible();
 expect(screen.getByText('保留适当留白')).toBeVisible();
 expect(screen.getByText(/未取得相关性排序，按已确认优先提供/)).toBeVisible();
 expect(screen.getByText(/未排除 1 条，已排除 1 条，未取得 0 条/)).toBeVisible();
 await userEvent.click(screen.getByText('人工确认回执'));
 expect(screen.getByText('approval-7')).toBeVisible();
});
it('旧版本、停用状态和缺失确认不会被新正文或当前确认覆盖',async()=>{
 render(<MemoryTimeline trace={{...trace,events:[{...trace.events[0],reads:[{...read,memoryTextVersion:'changed',passage:null,applicability:null,statusAtRead:null,selectionReason:null,disabled:true,confirmation:null}]}]}}/>);
 await userEvent.click(screen.getByText('请求中的记忆来路'));
 expect(screen.queryByText('保留适当留白')).not.toBeInTheDocument();
 expect(screen.getByText(/这条记忆的正文后来改过/)).toBeVisible();
 expect(screen.getByText(/已停用，不再参与读取/)).toBeVisible();
 expect(screen.getByText(/本次读取未记录确认回执/)).toBeVisible();
 expect(screen.getByText(/当时状态： 未记录或未取得/)).toBeVisible();
});
it('查询返回、空记录和缺失读取记录分开表述，并提示更早记录',async()=>{
 render(<MemoryTimeline trace={{hasEarlierEvents:true,events:[{...trace.events[0],kind:'query_result',round:null,reads:[]},{...trace.events[0],id:2,recorded:false,reads:[]}]}}/>);
 await userEvent.click(screen.getByText('请求中的记忆来路'));
 expect(screen.getByText(/记忆查询返回.*轮次未记录或未取得/)).toBeVisible();
 expect(screen.getByText('本条记录没有记忆条目。')).toBeVisible();
 expect(screen.getByText(/不能当作未提供记忆/)).toBeVisible();
 expect(screen.getByText(/另有更早记录/)).toBeVisible();
});
it('IPC 拒绝不匹配版本的正文、跨项目适用说明、来源不明的当前状态及矛盾计数',()=>{
 expect(memoryTraceSchema.parse(trace)).toEqual(trace);
 for(const bad of [{...read,memoryTextVersion:'changed'},{...read,scope:'cross_project'},{...read,memoryTextVersion:'source_unavailable',passage:null,applicability:null},{...read,evidence:{recorded:2,available:2,excluded:1,missing:0}}]){
  expect(()=>memoryTraceSchema.parse({...trace,events:[{...trace.events[0],reads:[bad]}]})).toThrow();
 }
});
