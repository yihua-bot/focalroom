# Lightroom Classic / Capture One 接入交付

日期：2026-10-22。用户要求：制作插件、官网提供下载及安装说明。

## 实际交付与发布状态

1. **Lightroom Classic 1.0.0 导出插件**：正确的 `FocalRoom.lrplugin/Info.lua` 注册导出服务，遍历真实 rendition、等待导出完成、绑定 Catalog 路径指纹及照片 localIdentifier；通过随包 macOS 通用组件计算原片与渲染 SHA、读取真实像素信息、生成完整批次。取消、导出失败、超过 500 张时不生成可导入包。旧的两个原型文件不参与构建。
2. **Capture One 1.0.0 macOS 桥接应用**：独立 `.app` 接收已经导出的 JPEG/PNG/单页 TIFF；逐张人工选择原片，然后保存新批次。不是 Capture One SDK 插件，不读 Catalog/Session/Variant，也不承诺这三者自动同步。可用访达“打开方式”，或在桥接应用里选择已导出的照片；不能把 Capture One 原图的 Open With 当作包含调色的导出。
3. **客户端接收入口（源码）**：选片工作台 → 调色版本 → 导入调色结果。原生系统选择器授权文件夹，按当前项目的原片 SHA 匹配。校验真实文件 SHA/尺寸/位深/格式、目录边界后，在同一事务内保存非当前调色分支与不可变渲染；同批次及同分支同内容重复导入跳过。正式照片、选片、序列、revision 不改变。接收副本与最长边 1400px 预览在应用自己的独立目录，重启后可查看。
4. **官网**：新增 `/plugins/`、`/en/plugins/`、`/ja/plugins/`、`/ko/plugins/`，完整安装/导出/导入/卸载/故障处理说明；页脚入口与 sitemap、canonical、hreflang、结构化页面信息同步。现有四语 16 页保留，合计 20 页。安装 ZIP 内含中英文说明和四语网页地址。
5. **正式签名与公证**：两个包均由 Developer ID Application 签名，arm64/x86_64 通用；Apple 均返回 Accepted。Capture One 应用已附公证票据，实际从最终 ZIP 解压后 codesign strict、stapler validate、Gatekeeper Notarized Developer ID 均通过。

**官网尚未发布。** 本机 Wrangler 凭据刷新返回 400；旧凭据的别名入口结果相同。最小权限 OAuth 已打开到官方登录页，但未收到授权并超时。没有新建 Cloudflare 项目、修改 DNS、改团队服务或绕到历史 Sites 托管。用户的本次上传请求已提供发布授权，下一次仅需恢复 Cloudflare 登录并确认现有 Pages 项目，不必再重复询问是否发布。

**本轮没有重打、发布 FocalRoom 正式客户端。** 插件可以配合旧版的普通照片导入使用，但会新增独立照片；按原片关联调色版本的入口必须随包含本轮最终源码的新客户端发布。公开 `https://focroom.cc/config.js` 本轮只读核对仍为 1.0.4，未改下载指针或旧客户端 ZIP。仓库期间外部 HEAD 推进到 `4ae4f17` 并收录部分先行源码，不代表该安装包已包含本轮最终修正。本聊天未执行 commit/push。

## 交付位置

- 源码：`between-frames/bridges/lightroom-classic/FocalRoom.lrplugin/`、`bridges/macos/Bridge.swift`。
- 构建：`between-frames/bridges/build.py`；构建默认不公证、不发布、不安装。`--sign` 使用已有 Developer ID；公证为独立步骤，不将凭据写入源码。
- 安装说明：`between-frames/bridges/INSTALL.md`。
- 最终包：`between-frames/bridges/build/`；官网源资源副本在 `film/focalroom-website/public/download/plugins/`，构建输出在 `dist/download/plugins/`。
- 网站部署 ZIP：`film/focalroom-website/artifacts/focalroom-website-plugins-20261022.zip`。只含当前静态 dist 文件；不含源码仓库、测试照片、数据库或凭据。
- 原生接收：`src-tauri/src/editor_bridge.rs`；前端：`EditorVersionsDialog.tsx`、`TauriEditorVersionsRepository.ts`。
- 验证材料：`evidence/editor-bridges-2026-10-22/`，最终下载 SHA 与公证 ID 另见下载包旁 `checksums.json`。

## 验证结果与范围

| 检查 | 结果 |
|---|---|
| 桌面前端全量 | 322/48 文件通过，含新增 4 项界面测试 |
| Rust 全量默认 | 285 通过、0 失败、16 显式 ignored |
| 最终下载包真实组件 → Rust 接收 | 单独启用 ignored 桥接测试，与其他 4 项一起共 5/5 通过；不是把 ignored 当作已运行 |
| Lua 5.1 | 正式 Info.lua、设置约束、真实本机组件导出、取消/失败/张数上限通过；中文、引号与 shell 字符路径参与验证 |
| Swift 组件 | 3 项真实可执行文件测试通过，覆盖两家 provider、实际字节/像素/原片指纹、重复新批次、缺原片/无效格式拒绝、未完成批次不发布、信息不含明文源路径 |
| 客户端接收 | 篡改 SHA/尺寸/源身份/provider/文件名、混合失败批次、符号链接越界均拒绝且无部分数据库记录；独立 provider/Catalog 分支；重复导入/重启和正式作品不变 |
| 原生桥接 GUI | 真实系统文件选择器添加 1 张生成照片、原片对应、保存到新目录与完成提示通过；不是 Capture One 真机验收 |
| 代码检查 | typecheck、lint、Clippy 严格、contracts:check、diff --check 通过，桌面前端生产构建通过 |
| 官网 | build、7 项语言测试、20 页静态校验、572 个本地引用及 20 个 sitemap URL 通过；CUA 核对四语页面、各 2 个下载入口、当前窄视口无横向溢出 |
| 包安全 | 两包 Apple Accepted；最终 Capture One ZIP 解压后签名、Gatekeeper、票据和下载 SHA 通过 |

中途确实发现并修复了列表查询误用 `p.name`（实际是 filename）、Lua 测试夹具错误替换 helper 路径导致未真正调用的问题；最终测试明确断言实际批次文件存在。一次额外复跑使用了错误相对 binary 路径而失败，随后改为最终 ZIP 解压出的绝对路径，5 项全部通过。没有用增加超时或放宽产品断言掩盖故障。

## 尚未完成与边界

- 本机没有 Lightroom Classic / Capture One。真实软件插件加载、实际导出及版本兼容矩阵、30 张往返尚未验收。Lua SDK 夹具、生成像素与本机桥接界面不等于两款软件真实验收。官网正文明确说明，不能宣传完整双向工作流。
- Capture One 原生 SDK 插件没有交付；本次交付的是明确标注的导出桥接应用。手动分支内多次渲染保留，不读取私有 Catalog 或伪造稳定 Variant ID。
- Lightroom Catalog 指纹基于路径；移动或改名 Catalog 会形成新的调色分支，未完成全局稳定 Catalog 身份接入。虚拟副本通过各自 localIdentifier 分开。
- 普通缩小预览沿用现有管线，不保证专业色彩转换、16-bit 显示、软打样或印厂验收；保留的渲染副本不改字节。
- 调色版本当前为单独查看，不自动替换正式作品序列，不回写评分/显影参数，不是在线协作同步。
- 失败批次可能留下应用私有目录内的无引用内容副本，数据库不部分写入；自动清理该目录不在此切片实现，不动用户原图。
- 网站公开发布等待 Cloudflare 授权。不能用本地 200、已备好下载文件或 Apple 公证来声称 `focroom.cc/plugins/` 已上线。

## 后续发布步骤

1. 用户在本机完成 `wrangler login`，不要把 token 发到聊天。只读获取现有 Pages 项目，确认自定义域名 `focroom.cc` 与生产分支。
2. 从现有 `film/focalroom-website` 构建并运行 check/test，再向该原项目上传 `dist`，保留部署 ID 与上一生产版本，不迁移 DNS。
3. 逐一核对四语公开页面、两个 ZIP 的实际字节与 SHA、原 1.0.4 下载不变、Google 验证文件、错误页与 sitemap。
4. 在有真实 Lightroom/Capture One 的机器先验少量副本，再运行 30 张重复导出/虚拟副本/离线源/文件名冲突/16-bit TIFF 旅程，保存版本号和原片前后 SHA。
5. 若发布带关联版本入口的新 FocalRoom 客户端，必须基于本轮最终来源重新验证、签名、公证与验收；不要复用外部先行提交的旧安装包冒充最终候选。

## 实现取舍

两款编辑器共用一个只读 Swift 批次生成器，Lightroom 只处理 SDK 导出流程，Capture One 桥接只处理人工对应。客户端使用独立仓库接口与单一接收模块，不增加运行时依赖或数据库迁移，不改旧回流函数的兼容语义。这样保持 KISS/DRY 和职责分离，不为未验证的 Capture One SDK 能力预留空实现。

官方依据：[Adobe 插件安装与导出](https://helpx.adobe.com/lightroom-classic/desktop/export-photos/export-presets-settings-plug-ins.html)、[Adobe Classic SDK](https://developer.adobe.com/lightroom-classic)、[Capture One 导出与 Open With / Edit With 区别](https://support.captureone.com/hc/en-us/articles/4403021653649-Export-overview)。
