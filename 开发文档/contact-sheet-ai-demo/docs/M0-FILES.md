# M0 真实文件清单

日期：2026-10-13。独立工程新增下列文件（包括官方模板基础文件和图标）；构建产物与依赖缓存不计入源码。原 Demo 和示例图未修改。

```text
between-frames/.gitignore
between-frames/.vscode/extensions.json
between-frames/README.md
between-frames/e2e/workspace.spec.ts
between-frames/eslint.config.js
between-frames/index.html
between-frames/package-lock.json
between-frames/package.json
between-frames/playwright.config.ts
between-frames/public/tauri.svg
between-frames/public/vite.svg
between-frames/src/App.css
between-frames/src/App.tsx
between-frames/src/app/AppController.tsx
between-frames/src/app/AppShell.tsx
between-frames/src/app/SideNav.tsx
between-frames/src/app/Workspace.tsx
between-frames/src/app/copy.ts
between-frames/src/app/router.ts
between-frames/src/app/useShortcuts.ts
between-frames/src/assets/react.svg
between-frames/src/components/compare/CompareModal.tsx
between-frames/src/components/contact-sheet/ContactSheetGrid.tsx
between-frames/src/components/inspector/PhotoInspector.tsx
between-frames/src/components/sequence/SelectionDock.tsx
between-frames/src/components/sequence/SequenceBoard.tsx
between-frames/src/components/shared/Button.tsx
between-frames/src/components/shared/EmptyState.tsx
between-frames/src/components/shared/Modal.tsx
between-frames/src/domain/models.ts
between-frames/src/features/ai-editor/AIPanel.tsx
between-frames/src/features/export/ExportDialog.tsx
between-frames/src/features/preference-memory/PreferencePanel.tsx
between-frames/src/features/project/ProjectDialog.tsx
between-frames/src/features/project/ProjectHeader.tsx
between-frames/src/features/project/ProjectList.tsx
between-frames/src/main.tsx
between-frames/src/services/MockRepository.test.ts
between-frames/src/services/MockRepository.ts
between-frames/src/stores/uiStore.ts
between-frames/src/styles/app.css
between-frames/src/styles/tokens.css
between-frames/src/test/App.test.tsx
between-frames/src/test/setup.ts
between-frames/src/vite-env.d.ts
between-frames/src-tauri/.gitignore
between-frames/src-tauri/Cargo.lock
between-frames/src-tauri/Cargo.toml
between-frames/src-tauri/build.rs
between-frames/src-tauri/capabilities/default.json
between-frames/src-tauri/icons/128x128.png
between-frames/src-tauri/icons/128x128@2x.png
between-frames/src-tauri/icons/32x32.png
between-frames/src-tauri/icons/Square107x107Logo.png
between-frames/src-tauri/icons/Square142x142Logo.png
between-frames/src-tauri/icons/Square150x150Logo.png
between-frames/src-tauri/icons/Square284x284Logo.png
between-frames/src-tauri/icons/Square30x30Logo.png
between-frames/src-tauri/icons/Square310x310Logo.png
between-frames/src-tauri/icons/Square44x44Logo.png
between-frames/src-tauri/icons/Square71x71Logo.png
between-frames/src-tauri/icons/Square89x89Logo.png
between-frames/src-tauri/icons/StoreLogo.png
between-frames/src-tauri/icons/icon.icns
between-frames/src-tauri/icons/icon.ico
between-frames/src-tauri/icons/icon.png
between-frames/src-tauri/src/lib.rs
between-frames/src-tauri/src/main.rs
between-frames/src-tauri/tauri.conf.json
between-frames/tsconfig.json
between-frames/tsconfig.node.json
between-frames/vite.config.ts
between-frames/vitest.config.ts
```

工程之外：

- 修改 `README.md`：追加工程启动与阶段说明。
- 新增 `DECISIONS.md`、`docs/USER-FLOW.md`、`docs/TEST-REPORT.md`、`docs/KNOWN_ISSUES.md`、`docs/M0-FILES.md`。
- 新增 `docs/screenshots/m0/{workspace,compare,sequence,empty,mobile}.png`：真实浏览器截图。
