# 出版物连续跨页编辑 · 2026-10-25

状态：本地实现和验收通过。仍是独立工作区，未合入或发行。完整计划不因此完成。

## 用户操作

1. 在摄影书的相邻左右内页中选中一张照片，点击「展开为跨页照片」。
2. 在预览中调整位置、尺寸、完整显示或裁切填满、焦点和放大倍数，确认后保存。
3. 点击左侧或右侧画面，都选择同一张照片。可拖动、使用四角或方向键调整，保存前只是预览。位置从左页左上角计算。
4. 「图像版本」改变整张跨页的来源；恢复版面版本时，左右画面及原来的来源一起恢复。
5. 「转为单页照片」先选择照片保留的页面和新构图。「移除照片框」会明确提示从两页一起移除，项目原片保留。
6. 两个关联页一起移动，先预览后保存。开本和出血一起修改，边距各页保留。会破坏配对的单页插入不可选，可以一次插入两页；删除会破坏跨页的页面前须先转为单页或移除照片。

## 实现边界与保护

- 一个正式 Frame、一份 Asset/Rendition、裁切和预览来源；对象归左页，右页只显示它的可见视窗。没有为右页复制第二张可独立漂移的照片。
- 左页 layoutJson 的 focalroomContinuousFramesV1 保存严格校验的 frameId/rightPageId。保留既有自定义设置及母版关联。
- 带连续照片的版面快照格式为2，普通版面仍为1；旧历史不重写摘要。当前兼容 pub_pages 入口遇到新格式会明确拒绝，不能把旧接口返回空白当作成功。未声称旧发行客户端能编辑新格式，发行前仍须验证版本回退策略。
- SetSpreadFrame/UnsetSpreadFrame 是明确的人工命令；继续使用当前基础版本、幂等编号、事务回执、持久待确认队列和不可变历史。回执不明时禁止新写入，只确认原请求。
- 同一项目/出版物、实体相邻偶数左页与下一奇数右页、相同开本和出血、来源及图像版本、装订线覆盖和边界均校验；右页投影参与重复检查。失败不留下局部写入或消费历史。
- 同页多选和自动排图只处理普通对象，独立跨页照片保持。单页母版不收录独立跨页照片；应用母版仍保留它。预览、缩略图、历史对比、文字编辑和照片替换均识别投影。
- 指针移动按当前物理单页的屏幕比例换算，约束范围按整个跨页计算；避免从右侧拖动时位移翻倍。
- 中英日韩文案同步。普通窗口与1024×768窗口截图已检查，非均匀照片的左右衔接已检查。
- 没有新增 SQL 表、迁移或依赖。未接触用户库、正式安装、生产、模型服务、钥匙串或 LUMIX；只用本地隔离 Store 和仓库示例照片派生素材。

## 验证

- 前端：493项 / 84文件通过。
- Rust库：169通过、2项Apple资源相关跳过；含新增6项跨页领域测试。不是完整Rust集成套件或原生验收。
- 定向实际浏览器：3条通过，覆盖连续显示和来源、从右侧移动、版本切换/恢复/重启、配对插页/整组移页/联动开本/转单页，以及丢回执重开和双页同时移除。
- 类型、Lint、契约生成一致性、fmt检查、严格Clippy、桌面前端和Web构建通过。构建仍有既有体积及混合静态/动态导入提醒，未以调高阈值消除。
- 全套本地浏览器：出版/工作区23、审稿9、旧画布4，共36条不同旅程通过；定向复跑不重复计数。

首轮失败保留：夹具最初只生成3页导致Rust新测试索引无效，改为通过正式生成命令产生4页后6项通过；一次过滤器拼错而运行0项，不计通过。浏览器初轮错误为测试断言传入数字以及保存按钮名称不匹配，均保留trace并修正；旧画布首次运行遗漏FOCALROOM_TEST_RPC_BASE导致4条连接错误，补齐1457地址后4条全过。未修改断言去绕过产品问题，未使用force点击或固定等待跳过失败。

证据目录：evidence/publication-continuous-2026-10-25/。verification.json记录源码、报告和产物SHA-256，可据此复核；320px版面预览不算生产像素、专业色彩或印刷验收。

## 尚未完成

H1仍有持久图层/分组、网格/标尺/章节/封面书脊、富文本/字体和生产文字检查。跨页照片不参与普通对象的混合成组调整；关联页的成组删除和跨批次拖动自动翻页未做。应用不同开本母版必须保持配对有效，不会自动拉伸独立跨页照片。

生产Edition、高分辨率渲染、PrintContract、PDF/X、原生WebView验收及完整A–K继续。此轮没有合入、提交、推送、部署、安装或发布。


## 复现入口

所有命令从本独立工作区的 between-frames 执行，浏览器前先完成构建，运行中不构建前端。

- npm test
- cargo test --manifest-path "src-tauri/Cargo.toml" --lib
- npm run typecheck / npm run lint / npm run contracts:check
- cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
- cargo clippy --manifest-path "src-tauri/Cargo.toml" --features integration-harness --lib --bin m1-harness -- -D warnings
- npm run build；npm run build --workspace @betweenframes/web
- cargo build --manifest-path "src-tauri/Cargo.toml" --features integration-harness --bin m1-harness
- npx playwright test --config "playwright.publication.config.ts"
- npx playwright test --config "playwright.editorial.config.ts"
- FOCALROOM_TEST_RPC_BASE="http://localhost:1457" npx playwright test --config "playwright.workspace.config.ts"

出版与旧画布共用1456/1457，必须串行。审稿使用1446/1447可独立运行。验收均使用隔离临时库，不能改成用户库。旧恢复界面与Agent旅程含本地模拟，不计真实模型或真人质量验收。

领域、渲染、表单解析和工作台交互分别集中在独立模块，共用既有命令/历史/队列，避免给左右页维护两套照片状态（KISS/DRY）。源文件入口：publication_spreads.rs、publicationContinuous.ts、PublicationContinuousDialog.tsx、PublicationPaper.tsx、PublicationStudio.tsx。
