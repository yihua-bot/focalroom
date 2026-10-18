import {expect,it,vi} from "vitest";
import {invoke} from "@tauri-apps/api/core";
import {TauriCloudAuthRepository} from "./TauriCloudAuthRepository";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn()}));
it("IPC白名单保留头像并移除任何会话或verifier字段",async()=>{
 vi.mocked(invoke).mockResolvedValue({authenticated:true,waiting:false,origin:"https://team.focroom.cc",user:{id:"u",name:"Author",email:"author@fixture.test",avatar:"https://lh3.googleusercontent.com/a/fixture",token:"private"},error:null,verifier:"private"});
 const result=await new TauriCloudAuthRepository().status();expect(result.user?.avatar).toBe("https://lh3.googleusercontent.com/a/fixture");expect(JSON.stringify(result)).not.toContain("private");
});
