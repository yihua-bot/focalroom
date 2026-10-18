import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AccountRecovery, readAccountLink, VerificationRequest } from "./AccountRecovery";
import { CloudError, request } from "../shared-api";

const state = vi.hoisted(() => ({ locale: "zh" }));
vi.mock("../i18n", () => ({ useWebI18n: () => state }));
vi.mock("../shared-api", async importOriginal => ({
  ...await importOriginal<typeof import("../shared-api")>(), request: vi.fn(),
}));
const token = "a".repeat(64);
beforeEach(() => { vi.mocked(request).mockReset(); state.locale = "zh"; history.replaceState(null, "", "/shared"); });

describe("账户恢复界面", () => {
  it("识别新旧邮件，区分无效令牌与普通共享地址", () => {
    expect(readAccountLink(new URL(`https://test.invalid/shared#reset-password=${token}`))).toEqual({ mode: "reset", token });
    expect(readAccountLink(new URL(`https://test.invalid/verify-email?token=${token}`))).toEqual({ mode: "verify", token });
    expect(readAccountLink(new URL("https://test.invalid/shared#verify-email="))).toEqual({ mode: "verify", token: "" });
    expect(readAccountLink(new URL("https://test.invalid/shared#project=p"))).toBeNull();
  });
  it("令牌移出地址且不自动消费；无效链接不显示可提交表单", () => {
    history.replaceState(null, "", `/shared#verify-email=${token}`);
    const view = render(<AccountRecovery link={{ mode: "verify", token }} onBack={vi.fn()} />);
    expect(location.href).not.toContain(token);
    expect(document.body.innerHTML).not.toContain(token);
    expect(request).not.toHaveBeenCalled();
    view.rerender(<AccountRecovery link={{ mode: "verify", token: "invalid" }} onBack={vi.fn()} />);
    expect(screen.getByRole("alert")).toHaveTextContent("链接无效");
    expect(screen.queryByRole("button", { name: "确认验证邮箱" })).not.toBeInTheDocument();
  });
  it("密码一致性校验、提交防重与成功状态均保留明确的人类操作", async () => {
    let finish!: () => void;
    vi.mocked(request).mockImplementation(() => new Promise(resolve => { finish = () => resolve({ reset: true }); }));
    render(<AccountRecovery link={{ mode: "reset", token }} onBack={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("新密码", { exact: true }), { target: { value: "new-password-123" } });
    fireEvent.change(screen.getByLabelText("再次输入新密码"), { target: { value: "different-password" } });
    const form = screen.getByRole("button", { name: "保存新密码" }).closest("form")!;
    fireEvent.submit(form);
    expect(screen.getByRole("alert")).toHaveTextContent("两次密码不一致");
    expect(request).not.toHaveBeenCalled();
    fireEvent.change(screen.getByLabelText("再次输入新密码"), { target: { value: "new-password-123" } });
    fireEvent.submit(form); fireEvent.submit(form);
    expect(request).toHaveBeenCalledTimes(1);
    expect(request).toHaveBeenCalledWith("/auth/reset-password", null, "POST", { token, password: "new-password-123" });
    finish();
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("密码已更新"));
    expect(screen.queryByLabelText("新密码", { exact: true })).not.toBeInTheDocument();
  });
  it("找回结果不声称账户存在，成功后切换语言不丢失状态", async () => {
    vi.mocked(request).mockResolvedValue({ sent: true });
    const view = render(<AccountRecovery onBack={vi.fn()} />);
    fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: "fixture@test.invalid" } });
    fireEvent.submit(screen.getByRole("button", { name: "发送重置邮件" }).closest("form")!);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("如账户存在"));
    state.locale = "en"; view.rerender(<AccountRecovery onBack={vi.fn()} />);
    expect(screen.getByRole("status")).toHaveTextContent("If the account exists");
    expect(request).toHaveBeenCalledTimes(1);
  });
  it("过期链接显示本地化错误且允许重新尝试", async () => {
    state.locale = "ja";
    vi.mocked(request).mockRejectedValueOnce(new CloudError("VALIDATION_ERROR", "server text"));
    vi.mocked(request).mockResolvedValueOnce({ verified: true });
    render(<AccountRecovery link={{ mode: "verify", token }} onBack={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "メールアドレスの確認を確定" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("期限切れ"));
    fireEvent.click(screen.getByRole("button", { name: "メールアドレスの確認を確定" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("確認しました"));
  });
  it("发送失败不显示已发送，韩语可区分已验证账户", async () => {
    state.locale = "ko";
    vi.mocked(request).mockRejectedValueOnce(new CloudError("EMAIL_UNAVAILABLE", "server text"));
    vi.mocked(request).mockResolvedValueOnce({ already_verified: true });
    render(<VerificationRequest session={{ id: "u", email: "u@test.invalid", name: "演练", csrf: "test" }} />);
    fireEvent.click(screen.getByRole("button", { name: "인증 이메일 보내기" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("보낼 수 없습니다"));
    fireEvent.click(screen.getByRole("button", { name: "인증 이메일 보내기" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("이미 인증된"));
  });
});
