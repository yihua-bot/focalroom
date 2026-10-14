# M1 真实文件变更清单

日期：2026-10-13。M0 已评审通过。没有 Git 提交或推送。

工程新增/修改文件：

```text
between-frames/src-tauri/Cargo.toml
between-frames/src-tauri/Cargo.lock
between-frames/src-tauri/build.rs
between-frames/src-tauri/src/lib.rs
between-frames/src-tauri/tauri.conf.json
between-frames/src-tauri/migrations/0001_local_core.sql
between-frames/src-tauri/src/domain.rs
between-frames/src-tauri/src/db.rs
between-frames/src-tauri/src/image_pipeline.rs
between-frames/src-tauri/src/importer.rs
between-frames/src-tauri/src/export.rs
between-frames/src-tauri/src/commands.rs
between-frames/src-tauri/tests/local_core.rs
between-frames/src-tauri/src/bin/m1-harness.rs
between-frames/src/domain/models.ts
between-frames/src/services/TauriRepository.ts
between-frames/src/App.tsx
between-frames/src/app/AppController.tsx
between-frames/src/app/Workspace.tsx
between-frames/src/app/SideNav.tsx
between-frames/src/app/router.ts
between-frames/src/app/copy.ts
between-frames/src/components/contact-sheet/ContactSheetGrid.tsx
between-frames/src/components/inspector/PhotoInspector.tsx
between-frames/src/components/compare/CompareModal.tsx
between-frames/src/features/project/ProjectList.tsx
between-frames/src/features/project/ProjectDialog.tsx
between-frames/src/features/photo-import/ImportDialog.tsx
between-frames/src/features/export/ExportDialog.tsx
between-frames/src/features/preference-memory/PreferencePanel.tsx
between-frames/src/styles/app.css
between-frames/src/test/App.test.tsx
between-frames/e2e/workspace.spec.ts
between-frames/e2e/local-core.spec.ts
between-frames/scripts/integration-server.mjs
between-frames/playwright.config.ts
between-frames/playwright.integration.config.ts
between-frames/eslint.config.js
between-frames/tsconfig.node.json
between-frames/package.json
between-frames/package-lock.json
between-frames/README.md
```

工程之外：

- 更新 `README.md`、`DECISIONS.md`、`docs/TEST-REPORT.md`、`docs/KNOWN_ISSUES.md`、`docs/USER-FLOW.md`。
- 保留 M0 报告至 `docs/M0-TEST-REPORT.md`。
- 新增本清单与 `docs/screenshots/m1/{workspace,compare,sequence,empty}.png`。
- 原 `demo-standalone.html`、`index.html`、原图 `assets/` 和三份产品需求正文没有修改。

原生补验追加：

- 新增 `docs/NATIVE-M1-TEST-REPORT.md`、`docs/native-source-checksums.json`、`docs/native-snapshot-before.json`、`docs/native-snapshot-after.json`。
- 新增 `docs/screenshots/m1-native/{workspace,compare,sequence,empty,import,export}.jpg`。
- 新增系统保存对话框实际输出 `docs/native-exports/m1-native-project.json`、`m1-native-index.csv`、`m1-native-contact-sheet.html`（仅测试素材，HTML 内嵌 20 张缓存缩略图）。
- 更新本清单、`docs/TEST-REPORT.md` 与 `docs/KNOWN_ISSUES.md`；本轮无实现代码改动。

构建产物、npm/Cargo 缓存、测试临时数据库及 200 张派生图库不计入源码；本机可查看 `between-frames/src-tauri/target/debug/bundle/macos/Between Frames.app`。

原生拖动修正追加（2026-10-13）：修改 `between-frames/src/components/sequence/SequenceBoard.tsx` 和 `between-frames/e2e/workspace.spec.ts`；新增 `docs/M1-DRAG-TEST-REPORT.md`、`docs/native-drag-snapshot-{before,after}.json`、`docs/screenshots/m1-native/drag-{fixed,restarted}.jpg`；更新上述验收文档和 `DECISIONS.md`。临时诊断代码已移除，Tauri 配置实验已恢复。
