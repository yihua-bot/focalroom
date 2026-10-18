import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AgentAnalysis } from "./AgentAnalysis";
import { request } from "../shared-api";

vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(),
  request: vi.fn(),
}));

const session = { id: "u1", email: "o@fr.test", name: "作者", csrf: "c" };
const state = vi.hoisted(() => ({
  consents: [] as Array<{ id: string; scope: string; endpoint_label: string; max_tasks: number; used: number; state: string }>,
  jobs: [] as Array<{ id: string; kind: string; status: string; error: string | null; created_at: string }>,
  results: [] as Array<{ id: string; task_type: string; summary: string; comment_count: number; cost_estimate_minor_usd: number; created_at: string; created_by_name: string }>,
}));

beforeEach(() => {
  vi.mocked(request).mockReset();
  state.consents = [];
  state.jobs = [];
  state.results = [];
  vi.mocked(request).mockImplementation(async (path: string) => {
    if (path.endsWith("/model-consents")) return { consents: state.consents, endpoint_label: "mock@host" };
    if (path.endsWith("/agent-jobs")) return { jobs: state.jobs };
    if (path.endsWith("/agent-tasks")) return { tasks: state.results };
    return {};
  });
});

describe("AI 分析评审分歧面板", () => {
  it("无授权时先引导授权；授权后可发起分析与撤回", async () => {
    render(<AgentAnalysis id="p1" session={session} />);
    await waitFor(() => expect(screen.getByText(/mock@host/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: "授权分析（5 次）" })).toBeInTheDocument();
    // 点击授权后刷新出 ACTIVE 授权。
    state.consents = [{ id: "c1", scope: "summarize_disagreement", endpoint_label: "mock@host", max_tasks: 5, used: 0, state: "ACTIVE" }];
    fireEvent.click(screen.getByRole("button", { name: "授权分析（5 次）" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "分析当前评论分歧" })).toBeInTheDocument());
    // 发起分析入队并显示任务状态与结果。
    state.jobs = [{ id: "j1", kind: "summarize_disagreement", status: "COMPLETED", error: null, created_at: "now" }];
    state.results = [{ id: "r1", task_type: "summarize_disagreement", summary: "一致保留克制开场；折中：双版本对比。", comment_count: 4, cost_estimate_minor_usd: 1, created_at: "now", created_by_name: "作者" }];
    fireEvent.click(screen.getByRole("button", { name: "分析当前评论分歧" }));
    await waitFor(() => expect(screen.getByText(/折中提案要点/)).toBeInTheDocument());
    expect(screen.getByText(/一致保留克制开场/)).toBeInTheDocument();
    // 撤回授权调用 DELETE。
    fireEvent.click(screen.getByRole("button", { name: "撤回授权" }));
    await waitFor(() =>
      expect(vi.mocked(request)).toHaveBeenCalledWith("/projects/p1/model-consents/c1", session, "DELETE"),
    );
  });
  it("额度用尽时禁用发起按钮", async () => {
    state.consents = [{ id: "c1", scope: "summarize_disagreement", endpoint_label: "m@h", max_tasks: 1, used: 1, state: "EXHAUSTED" }];
    render(<AgentAnalysis id="p1" session={session} />);
    // 无 ACTIVE 授权时回到授权引导，不显示已用尽的按钮。
    await waitFor(() => expect(screen.getByRole("button", { name: "授权分析（5 次）" })).toBeInTheDocument());
  });
});
