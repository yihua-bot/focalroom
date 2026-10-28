# 主线同步、CI补修与lane交接 · 2026-10-28

本轮在codex/integrate-creative-workspace同步已提交主线000dc03，保留既有画布/出版/审稿整合，并补修PR #2的CI问题。主checkout的0030_device_registry.sql及database/server/service、account-security/integration/min_privilege六个在途文件由另一agent负责，本轮不修改、不暂存它们，不刷新全局current-state快照。

## 实际状态

- 已核验main的run40655775922为completed/success，提交000dc0389f125b2b16e45bda5361ec6ca258f63a。
- PR #2已有be4410d整合提交及e792289主线冲突收口；原publication-versions目录仍保留237项WIP副本，但对应桌面增量已在远端分支，不能继续称为“全部未提交”。
- 这不表示PR已合入main。原be4410d CI确有源码检查错误；e792289的两次CI分别在setup-node下载阶段被90分钟job上限取消，没有执行后续测试。

## CI原因与修复

1. 原run40636602902在web-journey.mjs报3个getComputedStyle未定义。浏览器evaluate回调此前直接使用该全局，Node侧代码检查无法识别。改为通过DOM元素所属document.defaultView调用，正确限定浏览器作用域；不关闭检查、不加忽略规则。
2. run40636924494的setup-node在05:15:46Z开始、06:45:34Z取消；日志显示从nodejs.org下载25.8.1 arm64时长时间无进展并重试。它不是产品测试耗时。保持Node25.8.1和job90分钟上限，给依赖安装步骤独立10分钟限时，失败可及时定位；没有延长测试阈值或偷偷换运行时/下载镜像。
3. feature分支由pull_request触发检查，push仅对main触发，避免同一PR同时占用两套macOS检查。既有测试和独立应用构建步骤保持。

setup-node的工具缓存与下载机制参考[官方说明](https://github.com/actions/setup-node#supported-version-syntax)，网络失败结论来自本轮原始日志，未把网页说明当作本次运行成功证明。

## 同步与本地验证

同步main新增Web版本对比组件/契约/样式、对应2项测试与历史定位修复，保留本分支的提案过期提示、连续追加顺序和可读性修复。

- 前端589项/102文件通过；Lint、类型、契约、Desktop/Web前端构建通过。
- 真实本地PG/Web三会话综合旅程通过：差异预览不写、正确追加、缓存失效、实际对比度、部分采纳、历史定位、只读与撤权；零外部站点、零真实模型、零原图上传。
- 工作流YAML解析及事件/限时约束通过。
- 与上一批绑定摘要比对，247项Native/本地域源码和已验证harness二进制未变。本轮没有重跑237库Rust/5作者记忆HTTP/124PG，也没有把它们算作新的通过；此前结果保持各自的明确来源范围。
- 47出版/9审稿/4工作区旅程为上一批整合验收，本轮不重复计数，不将589前端与main393简单相加。

## 共享目录清理与交接

只将本聊天创建、未被跟踪且与PR内副本字节相同的INTEGRATION-REPLY-2026-10-28.md移到Documents/BetweenFrames-Reviews/coordination-2026-10-28保留备份；没有删除文档或改动main索引。摘要和移动记录见本轮evidence目录。其他在途文件和未跟踪内容未处理，证据写在本分支，不在共享checkout堆积残留。

G2三个工程接线决定、项目研究资料/Moodboard与Web审稿形态已经写在INTEGRATION-REPLY-2026-10-28.md，不需要继续等待产品形态输入才能做工程。正式经营主体、价格、税费/退款/售后承诺仍需真实商业输入；不代填、不开售。

本轮不部署、不发行、不处理旧真实模型UNKNOWN任务。远端CI新结果单独核对，未据本地通过冒称PR/main已全绿；完整G0–G8、Focal AI v2、v3.1与A–K保持。
