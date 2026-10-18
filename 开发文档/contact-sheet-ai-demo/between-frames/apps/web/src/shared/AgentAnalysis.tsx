import { useCallback, useEffect, useState } from "react";
import { request, type Session, errorMessage } from "../shared-api";

interface Consent {
  id: string;
  scope: string;
  endpoint_label: string;
  max_tasks: number;
  used: number;
  state: string;
}
interface Job {
  id: string;
  kind: string;
  status: string;
  error: string | null;
  created_at: string;
}
interface TaskResult {
  id: string;
  task_type: string;
  summary: string;
  comment_count: number;
  cost_estimate_minor_usd: number;
  created_at: string;
  created_by_name: string;
}

/** M5 AI 分析评审分歧：独立模型发送授权 → 排队云 Worker → 展示折中提案要点。 */
export function AgentAnalysis({ id, session }: { id: string; session: Session }) {
  const [consents, setConsents] = useState<Consent[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [results, setResults] = useState<TaskResult[]>([]);
  const [endpointLabel, setEndpointLabel] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      const consentReply = await request<{ consents?: Consent[]; endpoint_label?: string }>(
        `/projects/${id}/model-consents`,
        session,
      );
      setConsents(consentReply.consents ?? []);
      setEndpointLabel(consentReply.endpoint_label ?? "");
      const jobReply = await request<{ jobs?: Job[] }>(
        `/projects/${id}/agent-jobs`,
        session,
      );
      setJobs(jobReply.jobs ?? []);
      const taskReply = await request<{ tasks?: TaskResult[] }>(
        `/projects/${id}/agent-tasks`,
        session,
      );
      setResults(taskReply.tasks ?? []);
      setError("");
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [id, session]);

  // 首次与依赖变化时装载远端状态；setState 均发生在 await 之后。
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- 异步装载远端数据。
    void refresh();
  }, [refresh]);

  const active = consents.find(
    (consent) => consent.scope === "summarize_disagreement" && consent.state === "ACTIVE",
  );
  const run = async (path: string, body?: unknown) => {
    setBusy(true);
    setError("");
    try {
      await request(path, session, "POST", body);
      await refresh();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const revoke = async () => {
    if (!active) return;
    setBusy(true);
    setError("");
    try {
      await request(`/projects/${id}/model-consents/${active.id}`, session, "DELETE");
      await refresh();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const latest = results.find((result) => result.task_type === "summarize_disagreement");

  return (
    <section className="fr-agent-analysis" aria-label="AI 分析评审分歧">
      <h3>AI 分析评审分歧</h3>
      <p className="fr-muted">
        把项目评论交给云端模型（{endpointLabel || "未配置"}）分析分歧并给出折中提案要点；
        仅在你显式授权后发送，可随时撤回，不修改正式作品。
      </p>
      {error && <p role="alert" className="fr-error">{error}</p>}
      <div className="fr-inline">
        {!active && (
          <button disabled={busy} onClick={() => void run(`/projects/${id}/model-consents`, { scope: "summarize_disagreement", max_tasks: 5 })}>
            授权分析（5 次）
          </button>
        )}
        {active && (
          <>
            <button
              className="fr-primary"
              disabled={busy || active.used >= active.max_tasks}
              onClick={() => void run(`/projects/${id}/agent-tasks/summarize`, { kind: "summarize_disagreement", consent_id: active.id })}
            >
              分析当前评论分歧
            </button>
            <span className="fr-muted">额度 {active.used}/{active.max_tasks}</span>
            <button disabled={busy} onClick={() => void revoke()}>撤回授权</button>
          </>
        )}
        <button disabled={busy} onClick={() => void refresh()}>刷新</button>
      </div>
      {jobs
        .filter((job) => job.kind === "summarize_disagreement")
        .slice(0, 3)
        .map((job) => (
          <p key={job.id} className="fr-muted">
            任务 {job.status === "COMPLETED" ? "已完成" : job.status === "QUEUED" ? "排队中" : job.status === "RUNNING" ? "执行中" : job.status === "CANCELLED" ? "已取消" : "失败"}
            {job.error ? ` · ${job.error}` : ""}
          </p>
        ))}
      {latest && (
        <div className="fr-analysis-result">
          <strong>折中提案要点（{latest.comment_count} 条评论 · {latest.created_by_name}）</strong>
          <p>{latest.summary}</p>
        </div>
      )}
    </section>
  );
}
