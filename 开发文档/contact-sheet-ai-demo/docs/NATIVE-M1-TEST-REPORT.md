# M1 原生 macOS 补验

日期：2026-10-13（Asia/Shanghai）。用户回复继续后，Mac 已可访问。范围限于 M1 本地核心补验；未开发 AI 或进入 M2。

后续更新：原生鼠标拖动已修正并通过最终构建及完整重启验收，详见 [M1-DRAG-TEST-REPORT.md](M1-DRAG-TEST-REPORT.md)。下文保留首轮补验历史，`unconfirmed` 是当时结果。

实际应用：`between-frames/src-tauri/target/debug/bundle/macos/Between Frames.app`，Tauri / WKWebView，bundle ID `studio.betweenframes.desktop`。

项目：`M1 原生验收 · 示例派生图库`，ID `fd816d26-2e83-459e-82e0-06e4d776f55c`。图库为 200 张示例派生 JPEG，最长边 480px，唯一 COM 段确保指纹不同。不是摄影师真实图库，也不代表 200 张 20MB 照片性能。

## 结果

| 项目 | 操作与证据 | 结果 | 剩余问题 |
|---|---|---|---|
| 新建与空状态 | 原生窗口新建项目、保存创作意图 | passed | — |
| 系统导入 | macOS 文件夹选择器；新增 200、失败 0 | passed | 原生逐文件选择尚未单独补验，后端路径授权已有测试 |
| 图像显示 | 虚拟网格、序列、检查器和比较实际截图可见图像 | passed | 高分辨率/色彩性能未覆盖 |
| 人工选择 | K 保留、X 淘汰、Right 切换；20 keep / 1 reject / 179 undecided | passed | — |
| 备注 | 输入区按 k/x 只追加文字，未误改状态；显式保存 | passed | 草稿重启前需保存 |
| 比较 | 保存右图偏好及理由；选片数量不变；1 条比较记录 | passed | — |
| 按钮/键盘排序 | 下移；Space/Right/Space 把手排序；前三为 frame-002、003、000 | passed | — |
| 鼠标拖动 | 两次跨卡片拖动后顺序未变 | unconfirmed | 应用与自动工具原因未确定，须人工复核，不计通过 |
| 导出 | macOS 保存对话框，JSON / CSV / HTML；回执匹配实际 SHA256 | passed | 不含 PDF 排版 |
| 完整重启 | Cmd+Q；进程查询只剩 Vite；重新启动并打开项目 | passed | — |
| 持久化一致性 | 重启前后 projects / photos / photo_decisions / sequence_items / decision_events / comparison_events 完全相同 | passed | — |
| 原图只读 | 导入前与重启后的 200 个 SHA256 全部相同 | passed | 源夹具在临时目录，系统未来清理后需重新生成 |

保存备注：`原生验收：保留光线与画面停顿。kx`。末尾 kx 为输入焦点快捷键隔离测试。数据库统计：1 项目、200 photos、200 decisions、20 sequence_items、226 decision_events、1 comparison_event。

## 文件证据

- [源指纹清单](native-source-checksums.json)
- [重启前快照](native-snapshot-before.json)、[重启后快照](native-snapshot-after.json)：逐表内容相同，包含备注、事件和序列。
- [项目 JSON](native-exports/m1-native-project.json)：200 张引用、20 张序列及事件。
- [选片 CSV](native-exports/m1-native-index.csv)：201 行（含表头）。
- [接触印相 HTML](native-exports/m1-native-contact-sheet.html)：20 张内嵌 JPEG 缩略图。

| 导出 | 实际 SHA256（与 UI 回执一致） |
|---|---|
| JSON | `3e6f4e2dc57c4b0b35f98f8302a9687781096d3c3eda6b046701572e6bd86576` |
| CSV | `49273c6f2ed04e235d30e46c24c8852ec55054ad9753bcdd02ac79123e8f4f19` |
| HTML | `1f1f6c32a3ce0ad20ed17a228030b2e638b0640b5872f2579c6c8de933a4a29c` |

## 真实原生截图

- [工作台与重启后备注](screenshots/m1-native/workspace.jpg)
- [两图比较](screenshots/m1-native/compare.jpg)
- [作品序列](screenshots/m1-native/sequence.jpg)
- [空状态](screenshots/m1-native/empty.jpg)
- [导入结果](screenshots/m1-native/import.jpg)
- [导出回执](screenshots/m1-native/export.jpg)

本轮未修改代码，未重复已通过的 typecheck、lint、12 个 Vitest、9 个预览 E2E、1 个真实 Store 旅程、8 个 Rust 用例及 Tauri build；原始结果见 [TEST-REPORT.md](TEST-REPORT.md)。补验使用应用正常 IPC 写入数据，快照校验使用 SQLite 只读连接；没有工具直接写数据库、改动原图、Git 提交或推送。

原生鼠标拖动已通过后续修正验收；仍需有授权的摄影师真实 200 张图库。M1 验收尚有边界，继续停留在 M1。
