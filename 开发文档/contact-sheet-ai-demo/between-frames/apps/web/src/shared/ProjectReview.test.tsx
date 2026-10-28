import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ProjectReview } from "./ProjectReview";
import { request } from "../shared-api";

// 本 jsdom 环境未实现 Web Storage，用内存版替换。
const storage = new Map<string, string>();
vi.stubGlobal("localStorage", {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => void storage.set(key, value),
  removeItem: (key: string) => void storage.delete(key),
  clear: () => storage.clear(),
});

vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(),
  request: vi.fn(),
}));
const presenceState = {
  people: [] as { id: string; name: string; photo_id: string | null }[],
};
vi.mock("./useRealtime", () => ({
  useRealtime: () => ({
    online: true,
    people: presenceState.people,
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
  presenceState.people = [];
  storage.clear();
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

describe("协作者 Presence 展示", () => {
  it("显示协作者正在查看的照片并可隐藏动态", async () => {
    presenceState.people = [
      { id: "u2", name: "林编辑", photo_id: "p1" },
      { id: "u2", name: "林编辑", photo_id: "p1" },
    ];
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    const persons = screen.getAllByText("林", { selector: ".fr-person" });
    expect(persons).toHaveLength(1);
    expect(screen.getByText("p1.jpg", { selector: ".fr-person-viewing" })).toBeInTheDocument();
    expect(screen.getByTitle("林编辑 · 正在查看 p1.jpg")).toBeInTheDocument();
    fireEvent.click(
      screen.getByRole("button", { name: "隐藏协作者动态" }),
    );
    expect(
      screen.queryByText("p1.jpg", { selector: ".fr-person-viewing" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "显示协作者动态" }),
    ).toBeInTheDocument();
    expect(window.localStorage.getItem("focalroom.presence.detail")).toBe(
      "off",
    );
  });
  it("记忆上次隐藏状态", async () => {
    window.localStorage.setItem("focalroom.presence.detail", "off");
    presenceState.people = [{ id: "u2", name: "林编辑", photo_id: null }];
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    expect(
      screen.queryByText("林", { selector: ".fr-person" }),
    ).not.toBeInTheDocument();
  });
});

describe("提案差异预览", () => {
  const proposalSnapshot = () => {
    const base = makeSnapshot();
    return {
      ...base,
      snapshot: {
        ...base.snapshot,
        photos: [
          ...base.snapshot.photos,
          {
            id: "p2",
            filename: "p2.jpg",
            source_sha256: "b".repeat(64),
            selection: "UNDECIDED",
          },
        ],
      },
      proposals: [
        {
          id: "prop-1",
          author_id: "u2",
          author: "林编辑",
          title: "编排调整",
          base_revision: 3,
          revision: 0,
          nonce: "nonce-1",
          state: "SUBMITTED",
          operations: [
            {
              op: "add" as const,
              operation_id: "op-1",
              photo_id: "p2",
              to_index: 1,
            },
          ],
        },
      ],
    };
  };
  const diffResult = {
    proposalId: "prop-1",
    state: "SUBMITTED",
    baseRevision: 3,
    currentRevision: 3,
    stale: false,
    sequence: {
      id: "seq",
      before: ["p1"],
      after: ["p2", "p1"],
      added: ["p2"],
      removed: [],
      moved: [{ photo_id: "p1", fromIndex: 0, toIndex: 1 }],
    },
  };

  it("预览差异展示全量采纳后的序列变化（增/移）", async () => {
    vi.mocked(request).mockImplementation(async (path: string) => {
      if (path === "/projects/p/snapshot") return proposalSnapshot();
      if (path === "/projects/p/proposals/prop-1/diff") return diffResult;
      return {};
    });
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "提案" }));
    fireEvent.click(screen.getByRole("button", { name: "预览差异" }));
    await waitFor(() =>
      expect(
        screen.getByText(/全量采纳后序列：p2.jpg、p1.jpg/),
      ).toBeInTheDocument(),
    );
    expect(screen.getByText(/新增：p2.jpg/)).toBeInTheDocument();
    expect(
      screen.getByText(/移动：p1.jpg（第 1 张 → 第 2 张）/),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "刷新差异" }),
    ).toBeInTheDocument();
  });

  it("基线过期的提案显示不可投影提示", async () => {
    vi.mocked(request).mockImplementation(async (path: string) => {
      if (path === "/projects/p/snapshot") return proposalSnapshot();
      if (path === "/projects/p/proposals/prop-1/diff")
        return {
          ...diffResult,
          stale: true,
          baseRevision: 2,
          currentRevision: 3,
          sequence: null,
        };
      return {};
    });
    render(<ProjectReview id="p" session={session} onBack={() => {}} />);
    await waitFor(() =>
      expect(screen.getByText("正式作品序列")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: "提案" }));
    fireEvent.click(screen.getByRole("button", { name: "预览差异" }));
    await waitFor(() =>
      expect(screen.getByText(/基线过期/)).toBeInTheDocument(),
    );
  });
});
