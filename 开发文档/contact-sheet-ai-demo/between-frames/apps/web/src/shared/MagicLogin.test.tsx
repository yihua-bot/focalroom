import { beforeEach, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MagicLogin } from "./MagicLogin";
import { request, CloudError } from "../shared-api";

const state = vi.hoisted(() => ({ locale: "zh", t: () => "找回密码" }));
vi.mock("../i18n", () => ({ useWebI18n: () => state }));
vi.mock("../shared-api", async original => ({
  ...await original<typeof import("../shared-api")>(), request: vi.fn(),
}));
beforeEach(() => { vi.mocked(request).mockReset(); state.locale = "zh"; });
const enterEmail = () => {
  fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: "Editor@fixture.test" } });
  fireEvent.submit(screen.getByRole("button", { name: "发送验证码" }).closest("form")!);
};
it("邮件失败保留邮箱步骤，不显示验证码已发送", async () => {
  vi.mocked(request).mockRejectedValue(new CloudError("EMAIL_UNAVAILABLE", "邮件服务暂不可用"));
  render(<MagicLogin onLogin={vi.fn()} />); enterEmail();
  expect(await screen.findByRole("alert")).toHaveTextContent("邮件服务暂不可用");
  expect(screen.queryByLabelText("验证码")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "发送验证码" })).toBeEnabled();
});
it("连续提交只发送一次，确认按钮提交验证码并使用服务器CSRF", async () => {
  let resolve!: (value: unknown) => void;
  vi.mocked(request).mockImplementationOnce(() => new Promise(done => { resolve = done; }));
  const onLogin = vi.fn(); render(<MagicLogin onLogin={onLogin} />);
  fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: "Editor@fixture.test" } });
  const form = screen.getByRole("button", { name: "发送验证码" }).closest("form")!;
  fireEvent.submit(form); fireEvent.submit(form);
  expect(request).toHaveBeenCalledTimes(1);
  expect(screen.getByRole("button", { name: "处理中…" })).toBeDisabled();
  expect(request).toHaveBeenCalledWith("/auth/magic/request", null, "POST", { email: "editor@fixture.test" });
  resolve({ sent: true });
  const code = await screen.findByLabelText("验证码");
  fireEvent.change(code, { target: { value: "012345" } });
  expect(request).toHaveBeenCalledTimes(1);
  vi.mocked(request).mockResolvedValueOnce({ user: { id: "u", email: "editor@fixture.test", name: "Editor" }, csrf: "independent-csrf" });
  fireEvent.submit(screen.getByRole("button", { name: "验证并登录" }).closest("form")!);
  await waitFor(() => expect(onLogin).toHaveBeenCalledWith({ id: "u", email: "editor@fixture.test", name: "Editor", csrf: "independent-csrf" }));
  expect(request).toHaveBeenLastCalledWith("/auth/magic/verify", null, "POST", { email: "editor@fixture.test", code: "012345" });
});
it("验证失败保留重试，重新发码清除旧输入且不登录", async () => {
  vi.mocked(request).mockResolvedValueOnce({ sent: true });
  const onLogin = vi.fn(); render(<MagicLogin onLogin={onLogin} />); enterEmail();
  fireEvent.change(await screen.findByLabelText("验证码"), { target: { value: "123456" } });
  vi.mocked(request).mockRejectedValueOnce(new CloudError("VALIDATION_ERROR", "验证码错误或已过期"));
  fireEvent.submit(screen.getByRole("button", { name: "验证并登录" }).closest("form")!);
  expect(await screen.findByRole("alert")).toHaveTextContent("验证码错误");
  vi.mocked(request).mockResolvedValueOnce({ sent: true });
  fireEvent.click(screen.getByRole("button", { name: "重新发送" }));
  await waitFor(() => expect(screen.getByLabelText("验证码")).toHaveValue(""));
  expect(onLogin).not.toHaveBeenCalled();
});
it("验证码不足六位不提交，四语言切换保留邮箱步骤", async () => {
  vi.mocked(request).mockResolvedValue({ sent: true });
  const view = render(<MagicLogin onLogin={vi.fn()} />); enterEmail();
  fireEvent.change(await screen.findByLabelText("验证码"), { target: { value: "123" } });
  fireEvent.submit(screen.getByRole("button", { name: "验证并登录" }).closest("form")!);
  expect(screen.getByRole("alert")).toHaveTextContent("6 位");
  expect(request).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "更换邮箱" }));
  for (const [locale, name] of [["en", "Send code"], ["ja", "コードを送信"], ["ko", "코드 보내기"]]) {
    state.locale = locale; view.rerender(<MagicLogin onLogin={vi.fn()} />);
    expect(screen.getByRole("button", { name })).toBeEnabled();
    expect(screen.getByRole("textbox")).toHaveValue("editor@fixture.test");
  }
});
