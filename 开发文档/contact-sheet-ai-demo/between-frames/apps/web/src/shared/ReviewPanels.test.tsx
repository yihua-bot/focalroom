import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { HistoryPanel } from "./ReviewPanels";
import { request, type CloudSnapshot } from "../shared-api";

vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(),
  request: vi.fn(),
}));

const session = { id: "user-1", email: "e@fr.test", name: "编辑", csrf: "c" };
const photos: CloudSnapshot["snapshot"]["photos"] = [
  {
    id: "p1",
    filename: "p1.jpg",
    source_sha256: "a".repeat(64),
    selection: "KEEP",
  },
  {
    id: "p2",
    filename: "p2.jpg",
    source_sha256: "b".repeat(64),
    selection: "UNDECIDED",
  },
];

beforeEach(() => {
  vi.mocked(request).mockReset();
  vi.mocked(request).mockImplementation(async (path: string) => {
    if (path === "/projects/p/history")
      return [
        {
          revision: 1,
          created_at: "2026-10-28T00:00:00Z",
          command_id: "promotion:seed",
        },
        {
          revision: 2,
          created_at: "2026-10-28T01:00:00Z",
          command_id: "cmd-1",
        },
      ];
    if (path === "/projects/p/versions/1/diff/2")
      return {
        project_id: "p",
        from: { revision: 1, created_at: "2026-10-28T00:00:00Z" },
        to: { revision: 2, created_at: "2026-10-28T01:00:00Z" },
        photos: {
          added: ["p2"],
          removed: [],
          changed: [
            {
              id: "p1",
              fields: { selection: { before: "UNDECIDED", after: "KEEP" } },
            },
          ],
        },
        sequences: { added: [], removed: [], changed: [] },
      };
    return {};
  });
});

describe("版本间 Diff 对比", () => {
  it("选择两个版本后展示字段级差异", async () => {
    render(
      <HistoryPanel id="p" session={session} revision={2} photos={photos} />,
    );
    await waitFor(() =>
      expect(screen.getByText("v1", { selector: "strong" })).toBeInTheDocument(),
    );
    fireEvent.change(screen.getByLabelText("对比起始版本"), {
      target: { value: "1" },
    });
    fireEvent.change(screen.getByLabelText("对比目标版本"), {
      target: { value: "2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "对比差异" }));
    await waitFor(() =>
      expect(screen.getByText("v1 → v2")).toBeInTheDocument(),
    );
    expect(screen.getByText(/新增照片：p2.jpg/)).toBeInTheDocument();
    expect(
      screen.getByText(/p1.jpg：selection UNDECIDED → KEEP/),
    ).toBeInTheDocument();
  });

  it("未选齐或选择同一版本时禁止对比", async () => {
    render(
      <HistoryPanel id="p" session={session} revision={2} photos={photos} />,
    );
    await waitFor(() =>
      expect(screen.getByText("v1", { selector: "strong" })).toBeInTheDocument(),
    );
    expect(screen.getByRole("button", { name: "对比差异" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText("对比起始版本"), {
      target: { value: "1" },
    });
    fireEvent.change(screen.getByLabelText("对比目标版本"), {
      target: { value: "1" },
    });
    expect(screen.getByRole("button", { name: "对比差异" })).toBeDisabled();
  });
});
