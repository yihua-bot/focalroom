# 桌面云登录提交评审

后续修复与分阶段验收见 [修复报告](V2-M4-DESKTOP-LOGIN-FIX-TEST-REPORT.md)。以下保留原提交评审。

日期：2026-10-18。基线：`20a90ec`。本轮读取实现、已安装插件源码，并执行本地检查及隔离 PostgreSQL 认证验证；没有修改产品实现、注册系统协议、写入钥匙串、运行原生用户库、部署或提交/推送。

**结论：当前提交尚不能完成桌面云登录。** state 门卫已具备基本结构，协议配置存在；下面的调用链阻断仍需修复，不能以旧回归测试通过标记整个功能完成。

## 必须修复

1. **[P1] Web 回传 CSRF，桌面无法认证。** `apps/web/src/shared/MagicLogin.tsx:49` 将 `result.csrf` 放入 callback 的 token。`apps/api/src/auth.ts` 的 CSRF 为24随机字节的48位hex，session token为32字节的64位hex；Bearer只接受后者并查询其摘要。隔离实际认证证明CSRF返回401，真实session成功。不能简单把HttpOnly会话长期token放进深链；应增加短时、一次性授权码及桌面兑换验证，凭据在桌面后端保存。

2. **[P1] 发起 URL 不满足 Web 的回跳条件。** `CloudLoginView.tsx:49` 只传 `https://team.focroom.cc/shared`；`login_guard.rs:31` 只追加state，因此实际形状是 `/shared?state=...`。Web仅在 `desktop=1` 时回跳（MagicLogin.tsx:42）。按当前调用链，即使OTP成功，也不会进入桌面分支。

3. **[P1] 深链监听没有接通。** `src-tauri/src/lib.rs:28` 的Builder只初始化dialog，没有调用 `tauri_plugin_deep_link::init()`，也没有自定义深链转发。`CloudLoginView.tsx:23` 监听 `deeplink`，期待 `{path,params}`；本机安装的插件2.6.2实际发 `deep-link://new-url`，载荷为URL列表。代码库没有对应事件转换。需接通原生事件、严格解析 `focalroom://callback`，并处理已运行应用/冷启动回调，不能只增加依赖和scheme配置。

4. **[P1] 登录 UI、持久化及共享调用没有集成。** 全库检索只找到CloudLoginView的定义，没有挂载/导入使用；`commands.rs:658` 得到guard_token后仍TODO，直接返回true。`cloud_auth.rs` 没被lib.rs声明或其他模块使用；其中所谓钥匙串存储实际写SQLite设置，不能作为已生效存储。现有SharingControls仍提交email/password（:55），`shared_cloud.rs:135`仍调用密码登录，没有读取新会话。因此现在用户找不到新入口，共享流程也没有改用登录状态。

5. **[P1] 已有 Web 会话没有桌面完成流程。** SharedApp.tsx:57通过 `/auth/me` 恢复会话后直接展示项目；回跳逻辑只写在OTP组件verify中。即使补齐desktop参数，已登录浏览器打开桌面入口也不会回传。未来Google等方式也不会自动执行OTP私有回调，应把桌面授权完成流程接在统一的会话确认之后，并提供用户可重试的返回状态。

6. **[P2] HTTP回环判断使用前缀，且浏览器启动失败被忽略。** `login_guard.rs:65` 的starts_with允许 `http://localhost.attacker.example` 和 `http://127.0.0.1.attacker.example`；应按解析后的host精确匹配，校验认证来源并绑定到登录事务。`:79–84` 将open失败转换为false后丢弃，仍返回Ok，可能让界面一直等待。修复时一并验证失败回滚、取消、过期和重放。

## 关于协议注册与验证声明

`src-tauri/tauri.conf.json:47` 已声明deep-link.desktop.schemes中的focalroom，并非完全没有配置。仍需检查实际打包产物的CFBundleURLTypes和运行回调；本轮未打包或注册，不能断言macOS已通过，也不建议仅靠手写另一份Info.plist认定完成。

新增CloudLoginView还包含“支持Google”等未实现文案和直接硬编码中文；收口时同步四语，只描述已可用功能。它目前没有挂载，不影响已上线旧页面，但也不能作为可操作入口验收。

## 本轮实际验证

| 检查 | 结果 |
| --- | --- |
| npm run typecheck | exit0 |
| npm run lint | exit0 |
| cargo test --manifest-path src-tauri/Cargo.toml --lib login_guard --offline | 编译成功，0测试通过/0失败、25 filtered；有guard_token未使用warning，不是212项全套复跑 |
| MagicLogin.test.tsx | 4/4通过；现有测试不覆盖新增桌面回跳/深链流程，不等于237项全量复跑 |
| 临时PG AuthService实测 | csrf作为Bearer→401，真正session→接受；不输出任何token，0真实邮件/模型、0生产更改 |
| 源码/插件核对 | UI无调用、插件无init、事件名和载荷不匹配、共享仍走密码路径；插件事实来自本机锁定2.6.2源码 |

建议先完成安全兑换接口与调用链接线，再补覆盖发起→Web会话（新/已有）→回调→后端兑换/存储→UI恢复→共享复用的集成测试；之后用独立原生评审标识验暖启动、冷启动、过期/重放与打包协议。最后单独审阅Web发布和邮件配置，不能沿用上次仅针对固定修复镜像的部署批准。

证据：[evidence/v2-m4-desktop-login-review](evidence/v2-m4-desktop-login-review/README.md)。本评审没有把当前桌面功能或M4/M5标记为完成。
