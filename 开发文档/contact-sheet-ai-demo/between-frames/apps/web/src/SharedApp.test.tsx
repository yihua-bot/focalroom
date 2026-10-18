import { beforeEach, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SharedApp from "./SharedApp";

vi.mock("./i18n", () => ({ useWebI18n: () => ({ locale: "zh", t: (key: string) => key }), WebLanguageSwitcher: () => null }));
vi.mock("./shared/ProjectHome", () => ({ ProjectHome: () => <p>项目列表</p> }));
vi.mock("./shared/ProjectReview", () => ({ ProjectReview: () => <p>作品</p> }));
vi.mock("./shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("./shared-api")>(),
  request: vi.fn(async (path: string) => { if (path === "/auth/me") throw new Error("not signed in"); return { verified: true }; }),
}));
beforeEach(() => history.replaceState(null, "", "/shared"));

it("同页邮件链接在 popstate/hashchange 与令牌清理后仍显示操作界面", async () => {
  render(<SharedApp />);
  await screen.findByRole("heading", { name: "登录 FocalRoom" });
  const target = `${location.origin}/shared#verify-email=${"a".repeat(64)}`;
  history.pushState(null, "", target);
  fireEvent(window, new PopStateEvent("popstate"));
  expect(screen.getByRole("heading", { name: "验证邮箱" })).toBeInTheDocument();
  expect(location.hash).toBe("");
  fireEvent(window, new HashChangeEvent("hashchange", { newURL: target }));
  // 容器若再通知一次清理后的空地址，也不能丢弃当前内存令牌。
  fireEvent(window, new PopStateEvent("popstate"));
  expect(screen.getByRole("button", { name: "确认验证邮箱" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "返回登录" }));
  await screen.findByRole("heading", { name: "登录 FocalRoom" });
});

it("切换到另一个邮件链接时清空前一链接的完成状态", async () => {
  history.replaceState(null, "", `/shared#verify-email=${"a".repeat(64)}`);
  render(<SharedApp />);
  fireEvent.click(screen.getByRole("button", { name: "确认验证邮箱" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("邮箱验证完成"));
  const target = `${location.origin}/shared#reset-password=${"b".repeat(64)}`;
  history.pushState(null, "", target);
  fireEvent(window, new HashChangeEvent("hashchange", { newURL: target }));
  expect(screen.queryByRole("status")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "保存新密码" })).toBeInTheDocument();
});
