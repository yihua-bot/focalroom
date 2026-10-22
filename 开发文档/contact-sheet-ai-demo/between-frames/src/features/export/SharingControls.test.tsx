import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SharingControls } from "./SharingControls";
import type { SharingRepository, SharingStatus } from "../../domain/sharing";
const auth=(signed=true)=>({status:vi.fn(async()=>({authenticated:signed,waiting:false,origin:"https://team.focroom.cc",user:signed?{id:"u",name:"Author",email:"author@fixture.test"}:null,error:null})),open:vi.fn(),cancel:vi.fn(),logout:vi.fn(),updateProfile:vi.fn(),subscribe:vi.fn(async()=>()=>{})});
describe("桌面共享会话与上传授权",()=>{
 it("复用已登录账户，明确授权后迁移；请求不包含邮箱密码或token",async()=>{
  let completed=false;const repository:SharingRepository={auth:auth(),status:vi.fn(async():Promise<SharingStatus>=>({project_id:"p",authority:completed?"shared":"local"})),share:vi.fn(async():Promise<SharingStatus>=>{completed=true;return{project_id:"p",authority:"shared"};})};
  const changed=vi.fn();render(<SharingControls repository={repository} projectId="p" revision={4} onChanged={changed}/>);
  const submit=await screen.findByRole("button",{name:"开始共享"});await waitFor(()=>expect(screen.getByText("Author")).toBeVisible());expect(screen.queryByLabelText("API 服务地址")).not.toBeInTheDocument();expect(screen.queryByLabelText("账户密码")).not.toBeInTheDocument();expect(submit).toBeDisabled();fireEvent.click(screen.getByRole("checkbox"));expect(submit).toBeEnabled();fireEvent.click(submit);
  await waitFor(()=>expect(screen.getByRole("status")).toHaveTextContent("已开启在线审稿"));expect(repository.share).toHaveBeenCalledWith({projectId:"p",expectedRevision:4,baseUrl:"https://team.focroom.cc",webUrl:"https://team.focroom.cc",action:"resume",previewUploadAuthorized:true});expect(changed).toHaveBeenCalledOnce();
 });
 it("未登录或原迁移绑定不匹配时禁止上传，仍保留云端核验取消入口",async()=>{
  const repository:SharingRepository={auth:auth(false),status:vi.fn(async():Promise<SharingStatus>=>({project_id:"p",authority:"local-promoting",remote:{binding:{baseUrl:"https://old.example.com",webUrl:"https://old.example.com",userId:"u",workspaceId:"w"}}})),share:vi.fn()};
  render(<SharingControls repository={repository} projectId="p" revision={2}/>);await screen.findByText("共享尚未完成，可以继续或取消。");expect(screen.queryByLabelText("API 服务地址")).not.toBeInTheDocument();expect(screen.getByRole("checkbox")).toBeDisabled();expect(screen.getByRole("button",{name:"登录并共享"})).toBeEnabled();expect(screen.getByRole("button",{name:"取消共享"})).toBeDisabled();fireEvent.click(screen.getByRole("button",{name:"登录并共享"}));expect(await screen.findByRole("dialog",{name:"登录 FocalRoom"})).toBeVisible();expect(repository.share).not.toHaveBeenCalled();
 });
});
