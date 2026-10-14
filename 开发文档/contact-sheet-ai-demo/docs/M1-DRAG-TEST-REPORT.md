# M1 原生鼠标拖动修正与验收

日期：2026-10-13（Asia/Shanghai）。用户要求继续开发，先解决 M1-11。M2 未开始；真实用户图库仍未提供。

## 问题与修正

原生自动操作按下序列把手、跨卡片松开后，排序没有变化。临时事件诊断观察到：`down 485 488 | start | up 650 378 | end <active> <active>`，没有有效拖动位移。

安装版本的 dnd-kit PointerSensor 在首次超过 distance 阈值时直接 `return handleStart()`，该事件不会进入更新位移逻辑。把手已有独立交互区域，按下即启动拖动足够明确，因此移除原 6px 启动阈值，继续沿用 dnd-kit 排序、键盘、取消和持久化接口。

修正后的同一原生操作轨迹为 `down 485 488 | start | delta 165 -110 | up 650 378 | end <active> <target>`，排序实际改变。诊断代码已从最终构建移除。

曾试验关闭 Tauri 原生文件拖放处理器，单独修改不能解决问题，最终已恢复原配置。没有添加额外依赖或自定义拖动状态机（KISS/YAGNI）；排序继续统一走已有 Repository/事务（DRY/SRP）。

## 本轮验证

| 项目 | 验证方式 | 结果 | 剩余问题 |
|---|---|---|---|
| 单次移动 | 新增 Playwright 用例，仅发送一次鼠标移动 | passed | — |
| 取消 | 上述用例验证 Escape 后顺序和数量不变 | passed | — |
| 鼠标/键盘 | 完整预览 E2E，含原有多步鼠标、方向键、Space 把手排序 | passed | — |
| 原生鼠标 | 无诊断代码的最终 `.app`，第一张拖到第三张 | passed | — |
| 状态隔离 | 鼠标排序后仍为 200 photos / 20 keep / 1 reject | passed | — |
| 原生完整重启 | Cmd+Q 后确认进程退出；重新启动并打开序列 | passed | — |
| DB 一致性 | 重启前后六表只读快照完全相同；revision 228，228 个决策事件 | passed | — |
| 原图只读 | 再次校验原 200 个源 SHA256 全部不变 | passed | 仍为示例派生夹具 |
| 真实图库 | 等待用户提供约 200 张照片的本地路径 | pending | 不宣称摄影师图库或高分辨率性能通过 |

最终前三张：`frame-002.jpg → frame-000.jpg → frame-003.jpg`。应用保留该测试项目，可直接评审。

```text
npm run typecheck            passed
npm run lint                 passed，0 warnings
npm run test                 passed，2 files / 12 tests
npm run test:e2e             passed，10 tests / 10.7s
npm run tauri build -- --debug（命令级 Cargo 镜像缓存） passed，41.81MiB arm64 .app
```

最终 Vite 8.3.4 构建仍有约 526kB 单包提示。Rust/数据库实现未改动，本轮不重复已有 8 个 Rust 用例与真实 Store 浏览器旅程；本轮原生操作直接验证排序事务与重启。

## 变更与证据

- 修改 `between-frames/src/components/sequence/SequenceBoard.tsx`：PointerSensor 移除启动距离阈值，中文注释解释合并事件。
- 修改 `between-frames/e2e/workspace.spec.ts`：新增单次移动和 Escape 取消回归测试。
- 更新 `docs/TEST-REPORT.md`、`docs/KNOWN_ISSUES.md`、`docs/NATIVE-M1-TEST-REPORT.md`、`docs/M1-FILES.md`、`DECISIONS.md`；新增本记录。
- [鼠标排序后截图](screenshots/m1-native/drag-fixed.jpg)、[完整重启后截图](screenshots/m1-native/drag-restarted.jpg)。
- [重启前快照](native-drag-snapshot-before.json)、[重启后快照](native-drag-snapshot-after.json)，保留早期快照作为历史证据。

原工作台、比较、序列和空状态截图见 [NATIVE-M1-TEST-REPORT.md](NATIVE-M1-TEST-REPORT.md)。原图、产品规格、SQL migration、依赖锁和 Tauri 配置未修改；没有 Git 提交或推送。

按用户要求的里程碑顺序，仍需完成执行规范中“导入 200 张用户照片，关闭软件重启后状态仍然存在”的真实图库验收，再进入 M2。M2 范围仍为 Provider interface、MockProvider、Run 进度/取消、独立建议、显式接受/忽略与 schema 校验；真实视觉调用属于 M3。
