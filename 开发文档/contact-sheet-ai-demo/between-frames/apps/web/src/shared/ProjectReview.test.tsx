import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ProjectReview } from "./ProjectReview";
import { request } from "../shared-api";

vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(),
  request: vi.fn(),
}));
vi.mock("./useRealtime", () => ({
  useRealtime: () => ({
    online: true,
    people: [],
    note: "",
    updateNote: vi.fn(),
    viewPhoto: vi.fn(),
  }),
}));

const session = { id: "user-1", email: "e@fr.test", name: "编辑", csrf: "c" };
const makeSnapshot = () => ({
  project_id: "p",
  workspace_id: "w",
  title: "协作项目",
  state: "SHARED",
  revision: 3,
  cursor: "c0",
  snapshot: {
    photos: [
      {
        id: "p1",
        filename: "p1.jpg",
        source_sha256: "a".repeat(64),
        selection: "UNDECIDED",
      },
    ],
    sequences: [
      { id: "seq", title: "主序列", items: [{ id: "item-p1", photo_id: "p1" }] },
    ],
  },
  principal: { role: "EDITOR", can_approve: false, can_edit_formal: true },
  previews: [],
  comments: [
    {
      id: "c1",
      author_id: "user-1",
      reviewer: "编辑",
      body: "开场这张放在第一位更稳",
      anchor: {
        type: "sequence",
        sequence_item_id: "item-p1",
        revision_id: "3",
      },
      parent_id: null,
      state: "OPEN",
    },
    {
      id: "c2",
      author_id: "user-1",
      reviewer: "编辑",
      body: "这一版整体节奏偏快",
      anchor: { type: "revision", revision_id: "3" },
      parent_id: null,
      state: "OPEN",
    },
  ],
  proposals: [],
  grants: [],
});

beforeEach(() => {
  vi.mocked(request).mockReset();
  vi.mocked(request).mockImplementation(async (path: string) => {
    if (path === "/projects/p/snapshot") return makeSnapshot();
    return {};
  });
});

describe("序列与版本批注入口", () => {
  it("快照中的序列/版本批注单独成列并标明锚点", async () => {
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    expect(screen.getByText("序列与版本批注")).toBeInTheDocument();
    expect(screen.getByText("开场这张放在第一位更稳")).toBeInTheDocument();
    expect(screen.getByText("版本 v3 · 待处理")).toBeInTheDocument();
  });

  it("序列条目批注：入口→目标提示→发布携带 sequence 锚点并回到照片目标", async () => {
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "批注序列第 1 张" }));
    expect(screen.getByText(/目标：序列第 1 张 · v3/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("序列批注"), {
      target: { value: "这里节奏断了" },
    });
    fireEvent.click(screen.getByRole("button", { name: "发布批注" }));
    await waitFor(() => {
      const call = vi
        .mocked(request)
        .mock.calls.find(([path]) => path === "/projects/p/commands");
      expect(call).toBeTruthy();
      const payload = call![3] as { payload: { anchor: unknown } };
      expect(payload.payload.anchor).toEqual({
        type: "sequence",
        sequence_item_id: "item-p1",
        revision_id: "3",
      });
    });
    await waitFor(() =>
      expect(screen.getByText(/p1\.jpg · 0 条批注/)).toBeInTheDocument(),
    );
  });

  it("版本批注：入口→发布携带 revision 锚点", async () => {
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "批注当前版本 v3" }),
    );
    expect(screen.getByText(/目标：作品版本 v3/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("版本批注"), {
      target: { value: "整体偏紧" },
    });
    fireEvent.click(screen.getByRole("button", { name: "发布批注" }));
    await waitFor(() => {
      const call = vi
        .mocked(request)
        .mock.calls.find(([path]) => path === "/projects/p/commands");
      expect(call).toBeTruthy();
      const payload = call![3] as { payload: { anchor: unknown } };
      expect(payload.payload.anchor).toEqual({
        type: "revision",
        revision_id: "3",
      });
    });
  });
});
