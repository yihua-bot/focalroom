import {beforeEach,expect,it,vi} from "vitest";
import {render,screen,fireEvent,waitFor} from "@testing-library/react";
import {DesktopApproval,readDesktopRequest} from "./DesktopApproval";
import {request} from "../shared-api";
const locale=vi.hoisted(()=>({locale:"zh"}));
vi.mock("../i18n",()=>({useWebI18n:()=>locale}));
vi.mock("../shared-api",async original=>({...await original<typeof import("../shared-api")>(),request:vi.fn()}));
const session={id:"user",name:"Fixture",email:"fixture@test.test",csrf:"browser-csrf"};
const intent={requestId:"a".repeat(64),state:"b".repeat(64),valid:true};
beforeEach(()=>{vi.mocked(request).mockReset();locale.locale="zh";});
it("明确账户确认后只回传授权码，不包含CSRF或Bearer",async()=>{
 vi.mocked(request).mockResolvedValue({code:"c".repeat(64)});const returned=vi.fn();render(<DesktopApproval intent={intent} session={session} onReturn={returned}/>);
 expect(request).not.toHaveBeenCalled();expect(screen.getByText(session.email)).toBeVisible();fireEvent.click(screen.getByRole("button",{name:"继续并返回桌面端"}));
 await waitFor(()=>expect(returned).toHaveBeenCalledOnce());const url=new URL(returned.mock.calls[0][0]);expect(url.protocol).toBe("focalroom:");expect(url.host).toBe("callback");expect([...url.searchParams.keys()]).toEqual(["code","state"]);expect(url.searchParams.get("code")).toBe("c".repeat(64));expect(url.href).not.toContain(session.csrf);
 expect(request).toHaveBeenCalledWith("/auth/desktop/authorize",session,"POST",{request_id:intent.requestId,state:intent.state});
});
it("无效/重复参数不发授权请求，网络失败不显示已返回",async()=>{
 expect(readDesktopRequest(new URL("https://team.focroom.cc/shared"))).toBeNull();
 expect(readDesktopRequest(new URL(`https://team.focroom.cc/shared?desktop=1&request=${intent.requestId}&state=${intent.state}&state=other`))?.valid).toBe(false);
 vi.mocked(request).mockRejectedValue(new Error("网络错误"));render(<DesktopApproval intent={intent} session={session} onReturn={vi.fn()}/>);fireEvent.click(screen.getByRole("button",{name:"继续并返回桌面端"}));expect(await screen.findByRole("alert")).toHaveTextContent("网络错误");expect(screen.queryByRole("status")).not.toBeInTheDocument();
});
it("重复点击不并发颁发授权码，四语显示一致",async()=>{
 let done!:(v:{code:string})=>void;vi.mocked(request).mockImplementation(()=>new Promise(resolve=>{done=resolve;}));const view=render(<DesktopApproval intent={intent} session={session} onReturn={vi.fn()}/>);const b=screen.getByRole("button",{name:"继续并返回桌面端"});fireEvent.click(b);fireEvent.click(b);expect(request).toHaveBeenCalledOnce();done({code:"c".repeat(64)});await screen.findByRole("status");view.unmount();
 for(const [code,title] of [["en","Sign in to desktop"],["ja","デスクトップにログイン"],["ko","데스크톱 로그인"]]){locale.locale=code;const current=render(<DesktopApproval intent={intent} session={session}/>);expect(screen.getByRole("heading",{name:title})).toBeVisible();current.unmount();}
});
