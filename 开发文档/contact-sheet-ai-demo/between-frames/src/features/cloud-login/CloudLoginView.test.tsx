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
    updateProfile: vi.fn(async () => ({ authenticated: true, waiting: false, origin: 'https://t.f.cc', user: null, error: null })),
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
    fireEvent.click(await screen.findByRole("button", { name: "账户菜单" }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "退出登录" }));
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

it("命令不发送通知时仍主动刷新退出状态；断网也保留本机退出入口",async()=>{
 const f=fixture({authenticated:false,user:{id:"cached",name:"Cached",email:"cached@fixture.test"},error:"无法连接云端"});
 vi.mocked(f.repository.logout).mockImplementation(async()=>{f.current.value=status();return false;});
 render(<CloudAccount repository={f.repository}/>);
 fireEvent.click(await screen.findByRole("button",{name:"账户菜单"}));
 fireEvent.click(await screen.findByRole("menuitem",{name:"退出登录"}));
 expect(await screen.findByRole("alert")).toHaveTextContent("已在本机退出；云端会话未能撤销");
 expect(await screen.findByRole("button",{name:"登录云端"})).toBeVisible();expect(screen.queryByText("Cached")).not.toBeInTheDocument();
});
it("浏览器打开失败显示原因，并允许重试",async()=>{
 const f=fixture();vi.mocked(f.repository.open).mockRejectedValue(new Error("系统浏览器无法打开"));render(<CloudAccount repository={f.repository}/>);
 fireEvent.click(await screen.findByRole("button",{name:"登录云端"}));expect(await screen.findByRole("alert")).toHaveTextContent("系统浏览器无法打开");expect(screen.queryByRole("status")).not.toBeInTheDocument();
});
it("退出后迟到的状态请求不能恢复旧账户，卸载移除订阅",async()=>{
 const f=fixture();let resolve!:(value:CloudStatus)=>void;vi.mocked(f.repository.status).mockImplementationOnce(()=>new Promise(done=>{resolve=done;}));
 function Probe(){const {status:current}=useCloudStatus(f.repository);return <p>{current?.authenticated?"signed-in":"signed-out"}</p>;}
 const view=render(<Probe/>);await waitFor(()=>expect(f.repository.status).toHaveBeenCalledOnce());f.notify();await waitFor(()=>expect(f.repository.status).toHaveBeenCalledTimes(2));
 await import("@testing-library/react").then(({act})=>act(async()=>resolve(status({authenticated:true,user:{id:"old",name:"Old",email:"old@fixture.test"}}))));expect(screen.getByText("signed-out")).toBeVisible();
 view.unmount();f.notify();expect(f.repository.status).toHaveBeenCalledTimes(2);
});
