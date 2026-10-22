import { useEffect, useState } from 'react';
import type { AgentTask } from '../../domain/agent';
import { zh } from '../../app/copy';

/** 只展示已经记录的执行阶段和真实等待时长，不推算百分比或展示内部日志。 */
export function progressStage(task: AgentTask) {
  const step = task.steps[task.steps.length - 1];
  if (task.state === 'queued' || task.state === 'planning') return { label: zh.chatPreparing, at: step?.createdAt ?? task.createdAt };
  const labels: Record<string, string> = {
    vision_reserved: zh.chatReadingPhotos,
    vision_settled: zh.chatOrganizingSelection,
    model_attempt: zh.chatConsideringSelection,
    model: zh.chatOrganizingSelection,
    tool: zh.chatOrganizingSelection,
  };
  return { label: labels[step?.kind ?? ''] ?? zh.chatPreparing, at: step?.createdAt ?? task.createdAt };
}
export function TaskProgress({ task }: { task: AgentTask }) {
  const [now, setNow] = useState(Date.now);
  const active = ['queued', 'planning', 'running'].includes(task.state);
  useEffect(() => {
    if (!active) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [active]);
  if (!active) return null;
  const stage = progressStage(task);
  const since = Date.parse(stage.at);
  const seconds = Number.isFinite(since) ? Math.max(0, Math.floor((now - since) / 1000)) : null;
  return <div className="chat-task-progress" role="status">
    <p><span className="chat-progress-dot" aria-hidden="true" />{stage.label}</p>
    {seconds !== null && <p className="muted">{zh.chatStageWait.replace('{seconds}', String(seconds))}</p>}
    {seconds !== null && seconds >= 30 && <p className="muted">{zh.chatWaitingReply}</p>}
  </div>;
}
