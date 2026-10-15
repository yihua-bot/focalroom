# 评审修复：功能验收与剩余问题

日期：2026-10-14（Asia/Shanghai）。范围为评审发现的六项链路缺口，未扩展新版本功能。

## 功能验收

| 缺口 | 修复与验证 | 结果 |
|---|---|---|
| 模型工具回执 ID 丢失 | 从持久化 result.callId 读取；本地 HTTP 服务逐轮严格核对调用与回执，暂停/重启后仍关联完整；请求带输出上限 | 自动化通过 |
| 前端拒绝视觉预算/用量 | IPC 接受非零图像预算与费用，仍拒绝负数/超范围；保留原有请求幂等与业务冲突测试 | 自动化通过 |
| 新导出格式被桌面入口拒绝 | 共用格式映射：印刷参考保存 HTML，侧车选择父文件夹新建目录；旅程点击两个按钮，经真实 IPC/harness 写文件，断言 HTML、200 个侧车与私密 manifest；解锁后系统选择器与实际 PDF 补验通过 | 自动化及原生通过 |
| 上传后才检查预算 | 请求前持久化费用/配额预留；零费用与不足预算零上传，工具触顶不预取；暂停/超时/崩溃后恢复不重传，失败仍计量；异常服务商用量如实记录并暂停 | 自动化通过；真实计价待校准 |
| 单次 consent 可被复用 | StartTask 同事务绑定 task_id；重复绑定拒绝，隐式回退仅 project；旧 task 授权不跳过下一次确认，deny 写入后端 | 自动化通过 |
| 纠正随证据数变化失效 | 假设稳定身份兼容旧版次数后缀；增长合并原记录并保留停用；旧同义重复项受纠正阻断，关闭/重开数据库后仍不检索 | 自动化通过 |

模型请求同时带结构化项目/预算/记忆数据，并将其标注为非指令数据。云端账本复用现有 agent_steps，经 CommandHandler 写入，不增加表、不修改历史 migration。原图只读与正式编辑/草案隔离的既有回归仍通过。

## 最终验证

- Rust：59 passed；钥匙串写删回环 1 项未执行；性能基线保持 ignored，本轮不重测无关性能。
- 前端：11 files / 44 tests passed。
- 预览 E2E：16 passed。
- 真实 Rust/SQLite 集成 E2E：3 passed（M2 51.7s、补强 Beta 10.9s、M1 11.2s，合计 1.3m）。Beta 旅程已移除吞掉导出错误的 catch，并实际调用 test_restart 验证前后项目一致。
- contracts:check、typecheck、lint、fmt、严格 Clippy（含 integration-harness）：通过。
- 独立 Debug 评审应用构建：Between Frames Fix Review.app；identifier 为 studio.betweenframes.fixreview。工程仍为 0.1.0，不改变原应用 identifier。
- 自动化固定离线；视觉只调用本地模拟 HTTP 服务，不读取真实钥匙串密钥或调用生产 API。
- 解锁后补验：原生导入/导出/取消/重启与实际四页 PDF 通过；导出/册页 Rust 5 项回归通过；补强实际 PDF 页数后的 Beta 旅程单独复验 1 passed（52.3s，合计 54.4s）。

复现：

```bash
cd "开发文档/contact-sheet-ai-demo/between-frames"
export PATH="/opt/homebrew/bin:$PATH"
bash "scripts/verify-all.sh" --skip-keychain
CARGO_TARGET_DIR="target-v1" npm run tauri build -- --debug --config "src-tauri/tauri.fix-review.conf.json"
```

## 剩余问题与停止点

1. 锁屏阻塞已解除：系统保存、目录选择、取消、完整退出重启已补验；发现并修复 Safari 页脚多分页，实际最终 PDF 为 4 页 A4 横版，逐页检查通过。27 表重启前后完全一致，28 源 hash 不变，详见 [原生补验报告](NATIVE-REVIEW-FIXES-REPORT.md)。本轮另跑导出/册页 5 项回归、严格 Clippy、typecheck、lint 与 Debug 构建，并为 Beta 旅程增加实际 PDF 页数检查。
2. 真实模型 E2E、真实价格与图像 token 计量仍未验证。费用是工程估算，不能宣称任意网关的实际账单绝对硬封顶；异常报告用量会如实记账并暂停，见 ADR 0011。
3. 传输结果未知时保留预留；恢复不会自动重传，可能需要人类重新委托。这是避免隐性重复上传/费用的明确边界。
4. 本轮跳过钥匙串写删测试，不触碰真实密钥。签名/公证、真人 Beta、代表性高分辨率图库和既有 M4 产品缺口仍有效。
5. Debug 构建仍有主 JS 超过 500kB、Tauri core 动态导入无法分包的提示；本轮不做无关依赖或分包改造。

本轮不升级用户数据库、不提交或推送 Git。新增验收结束后停止，等待评审；决策见 [ADR 0011](ADR/0011-review-fixes.md)。
