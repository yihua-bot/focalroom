import {fireEvent,render,screen} from "@testing-library/react";
import {expect,it} from "vitest";
import {AccountAvatar} from "./AccountAvatar";
import {safeAvatarUrl} from "../../../packages/contracts/src/avatar";
it("无头像及非可信来源使用首字占位，不发起任意外部图片请求",()=>{
 for(const value of [null,"http://lh3.googleusercontent.com/a","https://lh3.googleusercontent.com.evil.test/p","https://evil.test/p","data:image/png,xx","https://user@lh3.googleusercontent.com/a"]){
  const view=render(<AccountAvatar name="摄影师" url={value} className="cloud-avatar"/>);expect(screen.getByText("摄")).toBeVisible();expect(view.container.querySelector("img")).toBeNull();expect(safeAvatarUrl(value)).toBeNull();view.unmount();
 }
});
it("可信头像加载失败回落首字，更新头像地址后可以重新加载",()=>{
 const view=render(<AccountAvatar name="Author" url="https://lh3.googleusercontent.com/a/one" className="cloud-avatar"/>);
 const image=view.container.querySelector("img")!;expect(image).toHaveAttribute("referrerpolicy","no-referrer");fireEvent.error(image);expect(screen.getByText("A")).toBeVisible();
 view.rerender(<AccountAvatar name="Author" url="https://lh3.googleusercontent.com/a/two" className="cloud-avatar"/>);expect(view.container.querySelector("img")).toHaveAttribute("src","https://lh3.googleusercontent.com/a/two");
});
it("本地已验证位图仅在桌面显式允许时展示，Web不接收data或SVG头像",()=>{
 const bitmap="data:image/png;base64,iVBORw0KGgo=";
 const view=render(<AccountAvatar name="Author" url={bitmap} className="cloud-avatar"/>);expect(view.container.querySelector("img")).toBeNull();
 view.rerender(<AccountAvatar allowEmbedded name="Author" url={bitmap} className="cloud-avatar"/>);expect(view.container.querySelector("img")).toHaveAttribute("src",bitmap);
 view.rerender(<AccountAvatar allowEmbedded name="Author" url="data:image/svg+xml;base64,PHN2Zz4=" className="cloud-avatar"/>);expect(view.container.querySelector("img")).toBeNull();
});
