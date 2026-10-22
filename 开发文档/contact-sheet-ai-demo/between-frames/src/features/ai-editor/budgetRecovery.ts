import { zh } from '../../app/copy';
import { defaultAgentBudget, type AgentTask } from '../../domain/agent';

const legacyBudget = /视觉请求预留超过|云端图像预算不足|后端预算已达到|工具预算耗尽|传输结果尚未确认|此前视觉请求失败|视觉等待达到时间上限/;
export function budgetMessage(task: AgentTask): string | undefined {
  if (!task.budgetIssue) return legacyBudget.test(task.reason) ? zh.chatLimitUnknown : undefined;
  return zh[{
    images: 'chatLimitImages', cost: 'chatLimitCost', steps: 'chatLimitSteps', rounds: 'chatLimitSteps',
    time: 'chatLimitTime', context: 'chatLimitContent', tokens: 'chatLimitContent',
  }[task.budgetIssue.kind]];
}

/** 只延长可恢复的执行额度，保留原有图片发送范围、授权与费用上限。 */
export function resumeBudget(task: AgentTask, duration: number) {
  if (task.budgetIssue && (!task.budgetIssue.requestNotSent || !['time','steps','rounds'].includes(task.budgetIssue.kind))) return null;
  if (!task.budgetIssue && legacyBudget.test(task.reason)) return null;
  const budget = { ...task.budget,
    maxModelTurns: Math.min(200, Math.max(task.budget.maxModelTurns,
      task.usage.modelTurns >= task.budget.maxModelTurns ? task.usage.modelTurns + defaultAgentBudget.maxModelTurns : 0)),
    maxToolCalls: Math.min(1000, Math.max(task.budget.maxToolCalls,
      task.usage.toolCalls >= task.budget.maxToolCalls ? task.usage.toolCalls + defaultAgentBudget.maxToolCalls : 0)),
    maxDurationSeconds: Math.min(7200, Math.max(task.budget.maxDurationSeconds, Math.ceil(task.elapsedMs / 1000) + duration)),
  };
  if (budget.maxModelTurns <= task.usage.modelTurns || budget.maxToolCalls <= task.usage.toolCalls || budget.maxDurationSeconds * 1000 <= task.elapsedMs) return null;
  return budget;
}
