# M4/M5：PostgreSQL 共享权威与协作评审验收

日期：2026-10-17。依据 v2 Implementation Kit 的 FINAL-PRODUCT-ENGINEERING-SPEC、EXECUTION-PLAN、ADR 0040，以及用户后续“把能推进的全部推进完成，一直推进到 M5”的指令。范围止于 M5，不开展 M6/M7。

## 结论与验证层级

**M4/M5 的主要工程链路已经实现，并在独立本地 PostgreSQL、真实 Rust 后端和独立浏览器会话中验收。生产与真人门槛仍未关闭，不能宣布最终计划全部完成。** M3 的专业质量、真实参与者与完整多 Adapter 工程不因推进共享 UI 自动通过。

首轮全量脚本 exit=0、三入口构建与独立 debug 打包通过；用户解锁后已执行原生系统导入/导出、迁移及只读重启，并补修界面标签和原生链接入口，最新验证见本文末尾。原始证据目录为 `evidence/v2-m4-m5-postgres/`。生成账号、生成图片及仓库示例图均为工程演练，不计真人摄影项目。

## 本轮交付

| 需求 | 实现与验收边界 |
|---|---|
| 云身份/Workspace/Grant/Invite | `apps/api` NestJS + 独立 PostgreSQL；scrypt 密码、摘要 session、24h 到期、Cookie/CSRF、短时登录限流；工作区普通成员不隐式获得项目权限，项目邀请邮箱绑定/7天/一次性接受/撤销，Owner/Admin 管理例外明确。手动分享邀请链接，无邮件发送 |
| 单一正式权威 | 严格 v2 CommandEnvelope、当前角色、项目行锁、expected_revision、幂等键/载荷/actor 一致性；真实操作、不可变版本、事件、完整命令载荷和回执同一事务。回执重放也重新检查当前 ACL |
| 受控私有预览 | 桌面仅从已登记派生缓存重编码 JPEG；服务再次校验 <=2048px/8MiB、去 EXIF/XMP/GPS。本地内容寻址存储与 R2 Adapter，原文件指纹与预览内容 SHA 分开。单次30秒网关票据绑定 session/项目/照片，读取前后检查 ACL；不开放裸桶链接 |
| 真实 Promotion | 桌面显式输入服务/账户并勾选预览同意，冻结本地写入，再上传/核验/云端提交/记录回执，进入 SHARED_CACHE。丢失确认/网络不确定/重启保持冻结，可重试；绑定端点/账户/工作区/清单摘要。已提交不能取消，只有云端确认 STAGING 已取消才恢复本地写入 |
| 共享包 | 预览 JPEG + 最小 manifest 的新目录导出，不包含原图、私有意图、作者记忆或密码。Web 导入明确建立独立副本，不伪称切换本地权威 |
| Web 审稿 | `/shared` 实际登录、邀请、项目列表/成员、预览、检查器、序列、照片锚点、线程/回复/解决状态；1440/1280 布局，图库每页48张按需取预览。项目链接、刷新、浏览器返回同步路由 |
| 编辑提案 | 自己的未提交草案仅作者可见，可版本校验修订；提交后逐项 Diff 人工批准。Reviewer/Viewer 不能审批，Editor 正式编辑与审批分别授权；空选驳回不推进正式版本。并发提案一个成功另一个保持过期原文，明确重建新草案 |
| 历史版本 | 不可变正式版本与命令事件可查看；历史作品只读，不隐式恢复或覆盖当前版本 |
| 实时/CRDT | WS 首帧一次性票据，持续检查当前 session/ACL，Presence、断线只读和重连权威 snapshot。Yjs 仅每用户每项目私人笔记，同用户窗口合并；正式序列、权限、审批拒绝 CRDT 旁路 |
| Agent 评论读取 | 5分钟 Task principal 绑定父 session/项目，只读原评论并保留 author/reviewer/anchor；撤权/到期/注销即拒绝，不能访问人类命令。实际 `workers/agent/src/comments.ts` 客户端，无模型调用或云作者记忆授权 |
| 部署准备 | 独立 API/Web Dockerfile、Nginx、隔离 Compose、环境示例、R2 配置和上线/备份/回退说明。生产依赖闭包在新临时目录安装并成功导入 API；不等于 Docker 镜像验收 |

## API、迁移与兼容

主入口是 `apps/api/src/server.ts` 的 `/api/v2`。共享契约在 `packages/contracts/src/cloud.ts`，Auth、Authority、PreviewStore、Realtime 分责；正式写入不经前端或 Yjs 直接改库。桌面新增 SharingRepository 和共享控制面板，沿用已有本地权威守卫。复用既有命令及格式策略，没有引入另一个正式编辑通道。

PostgreSQL 迁移为独立 `apps/api/migrations/0001_shared_authority.sql` 和追加的 `0002_audit_envelopes.sql`。`0001` 与已提交版本字节一致；`0002` 保存新命令完整 envelope、保护审计不可变。旧回执没有原请求的 envelope 保持 NULL，不推测或编造。迁移带版本/SHA/事务与 advisory lock；普通服务启动只核验，不自动 DDL。升级测试从只含 `0001` 的新临时库出发，保留旧回执再应用 `0002`。

旧 `cloud/` axum/SQLite 留作实验兼容。并发加入的 `/api/v2` 适配曾接受未验证账户、无效预览并直接产生 SHARED，不能作为正式权威；现已统一返回 `501 UNSUPPORTED_AUTHORITY`，不自动创建 v2 表。旧 v1 协议测试保持，但其占位业务不计 M4/M5 验收。修复前源码/测试已保存在证据目录的 `unreviewed-sqlite-v2-*.original.rs`，未回退外部提交。

## 测试及失败记录

一键复现：应用根目录 `bash "scripts/verify-all.sh" --skip-keychain`。测试进程清空真实模型 key，所有服务只绑定回环地址。临时 PostgreSQL runner 不读取生产连接配置，每次新 cluster、受控预览目录，结束关闭 cluster。S3/R2 签名测试仅使用本地协议服务器和测试凭据。

| 套件 | 验收重点 |
|---|---|
| 19项 PostgreSQL/存储测试 | Promotion 原子/重放、去元数据、私有媒体撤权、邀请/成员隔离、CSRF/错误输入；评论/回复项目边界，双提案并发、私有草案修订/部分批准/完整审计；WS/Yjs/重连/撤权；Task 权限/到期/父会话；重启、旧库追加迁移；S3签名/本地链接与内容碰撞/伪装损坏图片拒绝 |
| Web 三独立会话旅程 | author/reviewer/viewer，54张生成预览（含51张分页项目），实际登录/副本导入/邀请/坐标批注/私有草案修订/部分批准/历史版本/笔记重开/撤权；首屏最多48个照片节点、第二页3个；无浏览器错误/外域请求 |
| Rust Promotion 旅程 | 实际 Rust harness + NestJS + PostgreSQL，2张生成源图 hash 一致；共享导出、桌面表单 IPC、只读写拒绝、丢失确认、进程重启、错误端点重绑拒绝、已提交取消拒绝、原 revision 回执、显式安全取消 |
| 桌面与兼容回归 | GUI/Agent 契约、类型/Lint、Vitest、预览 E2E、桌面 Rust/fmt/严格 Clippy、旧云安全拒绝、6条真实后端旅程及 Web foundation |

Rust Promotion 旅程中的桌面表单由 Chromium + 实际 Rust IPC bridge 驱动，不把它写成系统文件选择器或 WKWebView 验收。原生应用另行记录。

保留首轮错误与修复依据：`verify-all-attempt1-typecheck.log`（测试 mock 类型）；`verify-all.log`/`legacy-cloud-attempt1-unsafe-expectations.log`（旧云严格检查与不安全测试预期）；`native-promotion-attempt1-selector.log`（关闭按钮定位）；`verify-attempt3-test-lint.log`（SQL测试转义）；`verify-final.log`（51张分页新增测试发现同页 hash 切项目不更新）。路由修复同步 hash/popstate、项目打开/返回地址，补刷新/返回回归；没有提高超时或掩盖失败。`web-route-paging.log` 保留修复后旧测试仍期待刷新退回列表的失败；`web-route-paging-final.log` 为修正刷新预期后的定向通过。最终全量额外等待第二页缩略图和检查器像素解码后截图。

页面证据：`login-1440.png`、`review-1440.png`、`review-1280.png`、`proposal-diff-1440.png`、`history-1280.png`、`revoked-1440.png`、`grid-page-1280.png`、`desktop-sharing-1440.png`、`desktop-readonly-1280.png`。构建不得与 Vite 浏览器测试并行，防止产物重写 HTML 触发页面刷新污染结果。

## 安全、隐私与回退

没有读取原用户数据库、LUMIX 目录或聊天中的真实密钥，没有真实模型请求、原图上传、生产 API/SSH/DDL 或云部署。生成账号/素材和临时库只服务本轮工程验收。共享不授予 AI/作者记忆访问；撤销不能收回已下载的像素，过往事件/版本/回执按审计保留。

失败 Promotion 保持本地冻结。用户应核对原服务端的权威状态，使用同一绑定重试；已提交不能通过修改本地状态恢复双写。部署回退保留原镜像/数据库/对象备份，先暂停新写再隔离验证，禁止删除历史或手工调整 revision。详见 `between-frames/ops/shared/README.md`。

工作期间 HEAD 从 `aa64110` 被外部进程推进至 `24462e0`；用户选择本聊天统一收口后核对，没有找到可识别的另一个仍写入本仓库的活动开发进程，未强制结束未知进程。本聊天没有 commit、push、reset；最终以源码 SHA 清单为准，不能把外部提交归为本聊天提交。

## NOT DONE

1. **M3**：>=30个80–300张的授权创作任务、六类/首次与第三次、专家与竞品同题、>=10位摄影师；完整 Anthropic/Gemini 等 Adapter 和跨服务文本/图像独立授权 fallback、质量驱动自动选模、真实成本/账单校准、全图库后台观察、云作者记忆专属授权与真实 Creator 隔离。既有 DeepSeek/同端点角色/已有事实索引不替代这些项。
2. **M4 生产**：真实容器构建、R2账户/权限/存储策略、TLS/域名、独立最小权限数据库账户、邮箱验证/账户恢复/集中限流、备份还原、监控与目标环境容量。Docker daemon 不可用；本轮没有登录腾讯云核验部署，不能沿用旧记录声称当前线上一定有或没有服务。
3. **M5**：>=5个真人双人项目、真实网络与多实例容量/异常；云模型 Worker/队列仍未实现。Task 评论读取已接客户端，不代表云模型已运行。桌面迁移后当前只保留初始只读快照，后续云版本请在 Web 查看。
4. **M0/M1 既有外部门槛**：远端 CI/评审记录、专业色彩/高像素与低端设备/WKWebView 总资源，按旧报告继续保留。

M6/M7 按本轮范围未开展，真实付费/服务订单不能计为 M5 的完成内容。没有给出无法证实的产品完成百分比。

## 最终收口记录

### 首轮完整验证与构建（原生续验前）

- `verify-complete.log`：`bash "scripts/verify-all.sh" --skip-keychain` **exit=0 / 全部通过**。187 Vitest；191桌面Rust通过、13显式ignored、1钥匙串回环按参数过滤；16浏览器预览、6真实后端旅程、1Web foundation；16旧云兼容/安全测试；19 PostgreSQL/存储测试；三会话Web旅程及实际Rust Promotion旅程均通过。契约、类型/Lint、fmt、桌面/旧云严格Clippy通过。ignored是真实Provider/素材/平台/性能等显式门槛，不算已执行成功。
- `build-root.log`、`build-workspaces.log`、`build-native.log` 均 exit=0。构建保留现有 chunk 超500kB、Tauri模块静态/动态同时引用的提示，没有通过调高阈值隐藏；真实低端性能仍按剩余计划补验。
- 新包：`between-frames/src-tauri/target-v1/debug/bundle/macos/FocalRoom Shared Review.app`，`studio.focalroom.v2sharedreview`，可执行文件59.66MiB；SHA `eb66197020963d4dc73dfafd72369b6c2c6e2e951f7d815b3b4b30bd0e297c71`。独立标识、图标与源文件一致、NaturalLanguage弱链接通过，见 `native-build.json`。这是本地debug评审包，未做发布签名/公证。
- 首轮冻结277个所列源/配置/依赖文件SHA一致（现保留为 `source-manifest-before-native-label-fix.json`，最新 `source-manifest-final.json` 已纳入接续修复），`final-integrity.json` 记录HEAD `24462e0309b476d3ecdaba1da08fe260d16ff0e6` 与0001迁移字节一致。路由修复及相关测试/两份README的预期变化已纳入该冻结清单，最终未再发现外部源修改。
- `golden-preservation.json`：90张既有基准PNG与本轮起始备份一致，其中29张测试重写后恢复。起始已有未提交PNG按起始字节保留，不用Git HEAD覆盖。新的协作证据单独保存；三会话54张生成预览、Rust2张源图没有AI请求/原图上传。

### 本地评审与原生状态

首个 `npm run review:shared` 进程在回合中断后停止；原日志 `review-local.log` 保留。已在新临时cluster重启，当前日志为 `review-local-native.log`，回环API `http://127.0.0.1:51891`。原生迁移项目评审入口：

`http://127.0.0.1:1430/shared#project=d6933e15-6f2e-4673-950d-6832e84f69d0`

仅本地演练账号：`author@focalroom.local` / `editor@focalroom.local`，密码 `review-local-2026`。另有8张仓库示例预览的演练项目（b4d2f27d-a9a9-40cf-9834-c4e82d31c5f6），评论和提案均标为演练；原生迁移另增加2张仓库演练预览与1张入选序列。此进程供本轮评审，停止即关闭临时cluster，不属于生产部署或真人双人项目。

原生新包已使用显式空模型key启动，其独立应用目录在启动前不存在；新临时库50表、0项目/0照片/0Agent任务，integrity_check=ok。初次CUA选择该应用时返回 **Mac is locked**；其后用户解锁，已继续原生验收，结果见接续记录。`native-window.json` 记录待办与2张仓库演练源图hash不变。不能把浏览器IPC旅程或打包成功当成这项已通过。


### 原生续验与范围收口（2026-10-17）

- 在独立 `studio.focalroom.v2sharedreview` 应用内，真实系统文件夹面板导入2张仓库演练照片：新增2、失败0、跳过0；保留1张形成序列。真实系统目录面板导出共享包，共2个 `.preview.jpg`、manifest、README，清单没有本机路径或私有创作意图。
- 真实原生表单在勾选预览授权前禁用迁移，确认后对独立本地NestJS/PostgreSQL完成迁移，服务端快照revision4、2照片/1序列项/2预览；桌面正式编辑入口禁用。完整Quit/新进程启动后50张表逐表行数/内容SHA一致，两张源图SHA不变、0Agent任务、数据库/WAL中没有演练密码。证据 `native-window.json`。
- 续验发现侧栏/项目列表把“桌面后端”误当成“LOCAL项目”，以及Tauri中普通 `target=_blank` 链接没有打开浏览器。修复复用既有sharing状态、列表明确为本机缓存；两个共享链接复用 `SharedReviewLink`，原生命令只接projectId，从已完成迁移绑定重建并核对地址，不接前端任意URL、不经Shell。当前系统浏览器打开实现仅macOS，其他原生平台明确显示复制地址提示。没有新增依赖或迁移。
- 保留额外失败：`native-final-verify.log` 中旧云HTTP注册曾返回502；本地夹具Client显式 `no_proxy` 后2项定向通过（`native-loopback-proxy-fix.log`），避免系统代理转发测试凭据。`native-final-resume.log` 中恢复项目后的旧 `LOCAL PROJECT` 文案断言失败；同步为缓存标签并增加“归档项目”按钮检查，原trace保留在 `native-label-local-core-failure/`。没有放宽断言超时或跳过失败。
- 原生工具首次连接遇到ScreenCaptureKit错误，重连应用成功。目录GoTo面板的键盘Return没有完成目标选择，最终通过可见补全项双击再点击Open完成；未使用脚本绕过系统文件选择器。

最终结果：

- `native-final-verify.log` 的契约/类型/Lint、188项Vitest、16项浏览器预览、192项桌面Rust（13 ignored/1钥匙串过滤）、fmt/严格Clippy通过；该次在旧云代理502处停止，失败如上保留。修复仅限旧云夹具Client直连，`native-final-resume.log` 中16项旧云/fmt/严格Clippy及harness重建通过，随后在旧列表断言处停止。断言修正后 `native-final-integration-cloud.log` **exit=0**，6条真实后端、19项PG/存储、三会话54预览Web旅程、真实Rust Promotion及1项Web foundation均通过。三份日志分段说明最终结果，不把被中断脚本写成完整exit=0。
- `native-complete-typecheck.log`、`native-complete-lint.log`、`native-complete-build-root.log`、`native-complete-build-workspaces.log`、`native-complete-build-native.log` 均exit=0。最终 `FocalRoom Shared Review.app` 可执行文件59.78MiB，SHA `f7d201be4837a840a47a8b6643f556ead17e78b1478d7a22727cc741b24ffc8b`，独立标识/图标及NaturalLanguage弱链接一致；`native-build.json` 更新为该包，先前构建记录另行保留。
- 最终 `source-manifest-final.json` **280**个所列文件一致、HEAD仍24462e0；新增原生链接组件/用例、追加本地旅程标签断言均纳入。90张起始PNG再次恢复并匹配；新续验截图在独立证据目录，不覆盖旧基准。
- 原生新包重新打开同一项目，侧栏“共享项目·只读缓存”、正式编辑入口禁用，50表继续逐表SHA一致。点击新的原生链接未显示错误；默认系统浏览器页面的CUA读取超时，**其页面截图/最终打开画面仍未计通过**。没有继续操作无关Chrome页面或改变浏览器设置；同一绑定URL已在IAB登录验收：v4、2照片、1序列项、660×440真实预览、实时连接。此限制不扩写成Mac仍锁定或迁移失败，也不冒充系统浏览器已取得截图。
- 原生/跨端证据：`native-sharing-ready.png`、`native-readonly-complete.png`、`native-migrated-web.png`；`native-window.json` 区分通过项与浏览器工具限制。原生系统浏览器打开仅实现macOS，其他原生平台提供复制地址提示。无新依赖、用户库升级、真实模型/照片外发、生产DDL/部署或Git写操作。

截至M5仍未完成的工程与真人/生产验收，以 [明确清单](V2-REMAINING-PLAN.md) 为准。M3完整Adapter/跨服务授权、云Worker/队列、生产身份恢复与观测性等确实仍需编码，不能概括成“只剩外部条件”。
