本WIP已由 PUBLICATION-CHAPTERS-2026-10-26.md 及最终verification.json替代。下列日志与session仅保留过程，现均已终止，勿按旧状态重复启动任务。

# 出版章节与可编辑章节页 · 开发中 · 2026-10-26

当前仍在 /Users/yihua/.codex/worktrees/focalroom-publication-versions/film 独立工作区；主工作区只读，未提交/合入/发布/安装/生产/模型/Keychain/LUMIX。完整G0–G8、Focal AI v2、v3.1、A–K持续，本轮不缩小最终目标。上一轮UI切片已完成538前端/48浏览器，见WORKSPACE-NAVIGATION-UX-2026-10-26.md。

## 本批已写代码（未完成最终验收）

- contracts/publication-chapters.ts: 严格Chapter{id,title,startPageId}，64章/120UTF16标题、唯一起始页；控制字符/行分隔/首尾空白拒绝，UUID用现有z.uuid。
- publication.ts新增独立SetChapters{chapters,pageIds}、CreateChapter{chapter,pages1..2,title:TextUpdate,startOnRight}；ReorderPages可选chapterChangesConfirmed，缺失不补值，null拒绝。
- Native publication_chapters.rs读写specJson.focalroomChaptersV1，以稳定起始内页界定范围；必须使用格式3及既有图层模型。初始化原默认图层，不新建SQL表/迁移。严格UUID/C1/BOM/行分隔与TS对齐；章节ID不撞页/对象/组，起始不能是封面、外页、重复或连续照片右半页。
- editing.reordered_rows抽纯元数据计划供既有SQLreorder和章节计划共用。SetChapters原子重排完整页序+更新章节；页面数据与预览相同时才跳过重排，旧页码空洞仍须按计划归一。第8项新Native测试专验这一点。
- CreateChapter先内部复用AddPage/AddText，再reconcile图层，校验右页和整本版本，一次历史/回执提交；标题是实际可编辑Text，空白陪页真实存在。失败回滚全部。
- RemovePage保护章节起始页；普通页排序改变归属必须显式true，版本绑定防旧许可；Generate.replaceExisting清理章节键且更新四语说明。新历史标签“调整章节”“新建章节页”四语映射。
- Native布局解析和TS checkPublicationScope均检查章节，损坏当前数据允许恢复有效冻结历史；旧历史不回写。
- TS domain/publicationChapters.ts解析/范围/归属变化；publicationChapterPlans.ts纯预览/整章移动及实际章节页payload，末尾插入取实际最后一页尺寸。
- PublicationChapterDialogs.tsx：章节管理在已有“页面”侧栏，非一级tab；现有页标记、改名/起始、拖动整章(DndKit鼠标/键盘)+按钮预览、全局保存/未保存保护；移除标记保留内容。新建章节页独立预览实际标题/空白两页，一页/两页和右页起始可选；已有跨页配对不拆；未填写标题不能提交占位。
- Studio接真实仓库/持久命令，章节导航支持冻结历史；排序影响列出并绑定当前版本+完整页序勾选，任何重新移动重置；起始页页面设置显示保护且不暴露失败删除按钮。创建后定位真实标题页。
- 中英日韩新增约47项文案，CSS含窄窗口/减少动态效果和模态层级。

## 验证进度

- 新Native最初7项已全过；最后增加第8项“旧页码空洞”，尚待全量重新跑。
- Native完整194+2资源跳过曾通过；之后仅加强RFC UUID校验并修复跳过重排条件/增加空洞测试，故需最终重跑预期195+2。
- 前端547项/94文件最终已通过（新增9：plans4/dialogs3/queue2）。最近最终日志 /tmp/focalroom-chapters-front-accepted.log；此后无前端代码变更。
- typecheck/lint/contracts/Desktop前端和Web构建曾通过，最终严格检查还须在当前源码补验。
- 最初Rust测试一处JS式逗号声明语法已修；TS未用Page和.at超target已修；一个前端测试import错目录已修；全量前端因普通reorder多传undefined而失败，产品改为无确认时保持4参数调用（不是放松旧断言）；原日志均保留在/tmp/focalroom-chapters-*.log。
- harness正在最新构建会话，工具session 37473，日志/tmp/focalroom-chapters-harness-browser.log。必须确认此进程终止/成功后再启动新浏览器。
- Playwright list已有38出版流程。新3旅程追加e2e/publication-studio.spec.ts末尾，尚未执行：真实章节标记/键盘拖整章/历史导航/重启；新章节页丢回执原请求确认；归属确认重置/起始页保护/移除标记不删内容。
- 新E2E已对照现有按钮“保存顺序”（不是保存页序）；若后续错定位/布局，请修真实问题，不force点击或固定sleep。首次失败保留trace至独立evidence再跑。

## 接着做

1. 核验harness最新构建成功、Native章节Set重排条件为 original.pages != next.pages（不是只比较IDs），第8项测试已加入。
2. 先跑新3旅程：FOCALROOM_TEST_EVIDENCE_DIRECTORY=../docs/v3/evidence/publication-chapters-2026-10-26/first-browser npx playwright test --config playwright.publication.config.ts --grep '真实章节：|真实章节页保存中断|章节归属确认绑定预览'。日志保存/tmp。浏览器期间不改源/构建前端。
3. 修问题后补全typecheck/lint/contracts/fmt/strictClippy/两前端构建；cargo test --lib（不跑外部Keychain测试）；若Native改了重建harness。
4. 完整出版38、审稿9（1446/1447可并行）、原工作区4（与出版同1456/1457必须串行且设FOCALROOM_TEST_RPC_BASE=http://localhost:1457）。检查1440x1000/1024x768新截图、来源/序列、原历史与幂等。
5. 报告最终事实，新manifest参照workspace-navigation-2026-10-26，以所有最终源码/报告/证据/harness SHA封存。更新AGENTS和FINAL-PLAN-REMAINING入口。
6. 章节范围真实批量删除、跨物理页组/母版图层、封面书脊/富文本、Edition/印前等以及A–K仍继续，完整专业出版未完成。勿将本地旅程当原生/生产/真人/印厂验收。勿调用旧UNKNOWN 300图任务或发真实模型请求。

## 12:44接续实况

最新Native完整195过、2Apple资源跳过；8新Native全量覆盖已包括旧页码空洞，RFC UUID/C1/BOM/行分隔校验亦已加入。前端547过、严格Clippy/fmt/typecheck/lint/contracts/Desktop+Web构建通过，harness最新完成日志/tmp/focalroom-chapters-harness-browser.log。
新3浏览器首次2过1失败：键盘ArrowUp后立即Space未等到目标状态；改成等待DndLiveRegion显示“移到章节 夜行 的位置”，此真实章节流程复验通过（/tmp/focalroom-chapters-browser-second.log），没有改产品拖动逻辑或force点击。
当前完整出版38正在session99424、日志/tmp/focalroom-chapters-publication-accepted.log；审稿9正在session42971、日志/tmp/focalroom-chapters-editorial-accepted.log。等终止后接原工作区4（端口串行）。
视觉QA已发现一个小问题待补：打开管理章节的Studio onClick没有先setNotice('')，带进上一条“版本已保存”提示，会与dirty章节预览混淆。当前浏览器运行期间不要编辑；完整回归终止后清这一个分支旧notice，并在新3章节流程补验此行为（可同时补新建页1024截图）。其余35旧出版旅程不触及该分支；如仅复验新3，最终报告清楚说明总51不同+章节3修订后复验，不虚称所有51在完全相同字节上重跑。
正式报告/最终hash清单仍未生成，不把本WIP计完成。全目标与所有后续缺口继续。
