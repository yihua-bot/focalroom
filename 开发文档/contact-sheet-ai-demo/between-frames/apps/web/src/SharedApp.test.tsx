import { beforeEach, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import SharedApp from "./SharedApp";
import {request} from "./shared-api";

vi.mock("./i18n", () => ({ useWebI18n: () => ({ locale: "zh", t: (key: string) => key }), WebLanguageSwitcher: () => null }));
vi.mock("./shared/ProjectHome", () => ({ ProjectHome: () => <p>项目列表</p> }));
vi.mock("./shared/ProjectReview", () => ({ ProjectReview: () => <p>作品</p> }));
vi.mock("./shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("./shared-api")>(),
  request: vi.fn(async (path: string) => { if (path === "/auth/me") throw new Error("not signed in"); return { verified: true }; }),
}));
beforeEach(() => {
 history.replaceState(null, "", "/shared");
 vi.mocked(request).mockReset().mockImplementation(async(path)=>{if(path==="/auth/me")throw new Error("not signed in");return {verified:true} as never;});
});

const desktopUrl=`/shared?desktop=1&request=${"a".repeat(64)}&state=${"b".repeat(64)}`;
const account={id:"fixture",name:"Author",email:"author@fixture.test",csrf:"private-csrf"};
it("已有浏览器会话进入桌面确认页，不跳到项目列表且不自动授权",async()=>{
 history.replaceState(null,"",desktopUrl);
 vi.mocked(request).mockResolvedValue(account);
 render(<SharedApp/>);
 expect(await screen.findByRole("heading",{name:"登录桌面端"})).toBeVisible();
 expect(screen.getByText(account.email)).toBeVisible();
 fireEvent.click(screen.getByRole("button",{name:"账户菜单"}));
 expect(screen.getByRole("menu")).toHaveTextContent(account.email);
 expect(screen.queryByText("项目列表")).not.toBeInTheDocument();
 expect(request).toHaveBeenCalledTimes(1);
});
it("验证码登录成功后保留桌面事务，统一进入账户确认页",async()=>{
 history.replaceState(null,"",desktopUrl);
 vi.mocked(request).mockImplementation(async(path)=>{
  if(path==="/auth/me")throw new Error("not signed in");
  if(path==="/auth/magic/verify")return {user:account,csrf:account.csrf} as never;
  return {sent:true} as never;
 });
 render(<SharedApp/>);
 await screen.findByRole("heading",{name:"登录 FocalRoom"});
 expect(screen.queryByText("使用 Google 登录")).not.toBeInTheDocument();
 fireEvent.change(screen.getByLabelText("邮箱"),{target:{value:account.email}});
 fireEvent.click(screen.getByRole("button",{name:"发送验证码"}));
 fireEvent.change(await screen.findByLabelText("验证码"),{target:{value:"123456"}});
 fireEvent.click(screen.getByRole("button",{name:"验证并登录"}));
 expect(await screen.findByRole("heading",{name:"登录桌面端"})).toBeVisible();
 expect(screen.queryByText("项目列表")).not.toBeInTheDocument();
 expect(request).not.toHaveBeenCalledWith("/auth/desktop/authorize",expect.anything(),expect.anything(),expect.anything());
});

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
