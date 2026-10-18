import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { CloudLoginView, CloudAccount, useCloudStatus } from "./CloudLoginView";
import type { CloudAuthRepository, CloudStatus } from "../../domain/cloud-auth";

function status(overrides: Partial<CloudStatus> = {}): CloudStatus {
  return { authenticated: false, waiting: false, origin: "https://team.focroom.cc", user: null, error: null, ...overrides };
}

function fixture(state: Partial<CloudStatus> = {}) {
  const current = { value: status(state) };
  const listeners = new Set<() => void>();
  const repository: CloudAuthRepository = {
    status: vi.fn(async () => current.value),
    open: vi.fn(async () => { current.value = status({ waiting: true }); listeners.forEach(fn => fn()); }),
    cancel: vi.fn(async () => { current.value = status(); listeners.forEach(fn => fn()); }),
    logout: vi.fn(async () => true),
    subscribe: vi.fn(async (callback: () => void) => { listeners.add(callback); return () => listeners.delete(callback); }),
  };
  return { repository, current, notify: () => listeners.forEach(fn => fn()) };
}

describe("桌面云登录 UI", () => {
  it("未登录时提供登录入口并进入等待态", async () => {
    const { repository } = fixture();
    render(<CloudLoginView repository={repository} onClose={vi.fn()} />);
    const button = await screen.findByRole("button", { name: "登录云端" });
    fireEvent.click(button);
    await waitFor(() => expect(screen.getByRole("status")).toBeTruthy());
    expect(repository.open).toHaveBeenCalled();
  });

  it("等待态可取消回到入口", async () => {
    const { repository } = fixture({ waiting: true });
    render(<CloudAccount repository={repository} />);
    fireEvent.click(await screen.findByRole("button", { name: "取消" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "登录云端" })).toBeTruthy());
    expect(repository.cancel).toHaveBeenCalled();
  });

  it("已登录显示账户并提供退出", async () => {
    const { repository } = fixture({ authenticated: true, user: { id: "u1", email: "a@b.c", name: "摄影师" } });
    render(<CloudAccount repository={repository} />);
    expect(await screen.findByText("摄影师")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "退出登录" }));
    await waitFor(() => expect(repository.logout).toHaveBeenCalled());
  });

  it("useCloudStatus 订阅后端更新", async () => {
    const { repository, current, notify } = fixture();
    let seen: CloudStatus | null = null;
    function Probe() { seen = useCloudStatus(repository).status; return null; }
    render(<Probe />);
    await waitFor(() => expect(seen).toBeTruthy());
    current.value = status({ authenticated: true, user: { id: "u1", email: "a@b.c", name: "新" } });
    notify();
    await waitFor(() => expect(seen?.authenticated).toBe(true));
  });
});
