import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { KnowledgeSearch } from "./KnowledgeSearch";
import { request } from "../shared-api";

vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(),
  request: vi.fn(),
}));

const session = { id: "u1", email: "o@fr.test", name: "研究者", csrf: "c" };

const photographerResult = {
  kind: "photographer", id: "p1", title: "Stephen Shore",
  subtitle: "1947–", photographer: null, reviewStatus: "verified", relevance: 1,
};
const seriesResult = {
  kind: "series", id: "s1", title: "Uncommon Places",
  subtitle: "1982", photographer: "Stephen Shore", reviewStatus: "verified", relevance: 0.8,
};

beforeEach(() => {
  vi.mocked(request).mockReset();
  vi.mocked(request).mockImplementation(async (path: string) => {
    if (path.startsWith("/knowledge/search")) return [photographerResult, seriesResult];
    if (path === "/knowledge/photographers/p1") return {
      id: "p1", name_en: "Stephen Shore", birth_year: 1947, death_year: null, movement: "新彩色摄影",
      series: [{ id: "s1", title_en: "Uncommon Places", year_start: 1982 }],
      claims: [{ claim_type: "capture_format", claim_text: "Uncommon Places 使用 4x5 大画幅 Crown Graphic。", review_status: "verified" }],
      sources: [{ url: "https://www.moma.org", institution: "MoMA" }],
    };
    if (path === "/knowledge/series/s1") return {
      id: "s1", title_en: "Uncommon Places", claims: [], sources: [],
    };
    return {};
  });
});

describe("摄影研究 Explore 面板", () => {
  it("搜索后展示跨实体结果并带类型徽标", async () => {
    const { container } = render(<KnowledgeSearch session={session} />);
    fireEvent.change(screen.getByLabelText("知识库搜索"), { target: { value: "Stephen Shore" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await waitFor(() => expect(screen.getByText("Uncommon Places")).toBeInTheDocument());
    expect(container.querySelector(".fr-knowledge-kind[data-kind='photographer']")).not.toBeNull();
    expect(container.querySelector(".fr-knowledge-kind[data-kind='series']")).not.toBeNull();
    expect(vi.mocked(request)).toHaveBeenCalledWith(
      "/knowledge/search?q=Stephen+Shore", session,
    );
  });

  it("点击结果展开详情：系列/断言/来源", async () => {
    render(<KnowledgeSearch session={session} />);
    fireEvent.change(screen.getByLabelText("知识库搜索"), { target: { value: "shore" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await waitFor(() => expect(screen.getByText("Uncommon Places")).toBeInTheDocument());
    fireEvent.click(screen.getAllByText("Stephen Shore")[0]);
    await waitFor(() => expect(screen.getByText(/4x5 大画幅/)).toBeInTheDocument());
    expect(screen.getByText("Uncommon Places（1982–）")).toBeInTheDocument();
    const source = screen.getByRole("link", { name: "MoMA" });
    expect(source).toHaveAttribute("href", "https://www.moma.org");
  });

  it("类型过滤将 kind 参数附加到搜索请求", async () => {
    render(<KnowledgeSearch session={session} />);
    fireEvent.change(screen.getByLabelText("知识库搜索"), { target: { value: "series" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await waitFor(() => expect(screen.getByText("Uncommon Places")).toBeInTheDocument());
    fireEvent.click(screen.getByRole("tab", { name: "摄影书" }));
    await waitFor(() =>
      expect(vi.mocked(request)).toHaveBeenCalledWith(
        "/knowledge/search?q=series&kind=photobook", session,
      ),
    );
  });

  it("无结果显示空态提示", async () => {
    vi.mocked(request).mockImplementation(async (path: string) =>
      path.startsWith("/knowledge/search") ? [] : {});
    render(<KnowledgeSearch session={session} />);
    fireEvent.change(screen.getByLabelText("知识库搜索"), { target: { value: "不存在的条目" } });
    fireEvent.click(screen.getByRole("button", { name: "搜索" }));
    await waitFor(() => expect(screen.getByText(/没有找到/)).toBeInTheDocument());
  });
});
