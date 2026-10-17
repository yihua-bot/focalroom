# FocalRoom 品牌接入（2026-10-17）

用户确定最终产品名称为 FocalRoom；标志使用提供图片上方的图案，去掉文字和白色背景。

## 当前结果

- Desktop/Web/根入口标题、侧栏产品名、弹窗、项目列表、实验审稿页和印刷参考署名已更新。
- 默认 Tauri 产品名为 FocalRoom，当前独立评审配置为 FocalRoom Review。保留原 identifier、数据库位置、钥匙串服务名、环境变量、包名和导出数据格式，避免改名触发新的数据路径。
- 历史需求包、验收截图和旧评审配置保留原名；品牌变化不改变 M3 partial 状态或后续计划。
- 用户明确允许本地 Python 后，直接从原图提取了图案；内置图像工具的三次结果均未采用。原图保持不变。
- 正式透明资源为 between-frames/public/focalroom-mark.png：480×480 RGBA，图案原始像素范围 398×384，保留九个部件与原比例，文字完全排除。内部不透明像素与原图一致；仅轮廓窄带解除白底预乘并移除独立纸纹像素。159,276 个全透明像素、5,477 个过渡透明像素，负空间也透明；白/深/灰背景已目视检查。
- 根入口、Desktop、Web 共用一份 public 资源；工作区 Vite 配置复用根 publicDir，侧栏与 favicon 使用同一路径，避免维护多份图片。侧栏以浅色 UI 底板保证深色图案可辨识，PNG 本身没有底板。
- Tauri 标准 PNG、ICO、ICNS 已从透明原稿生成；额外移动平台生成物留在临时目录，没有引入移动端工程。
- 独立 debug 应用 FocalRoom Review.app 已构建（59.14 MiB），Info.plist 的名称与图标摘要核验通过，identifier 保持 studio.betweenframes.m1reviewfixes。没有启动应用或打开用户库。

## 验证

已通过类型检查、Lint、Desktop/Web 构建、180 项前端单测和 5 项 Rust 印刷/侧车/册页回归测试。构建保留原有大包与无效动态 import 提示。

最终 16 项预览 E2E 与 1 项 Web E2E 通过，新增覆盖标题、logo 实际解码、收起导航后仍显示，以及 favicon 返回 PNG。首次 Web favicon 使用仓库外相对路径，开发服务器返回 HTML；改为共享 publicDir 后完整复验通过，失败 trace 单独保留。根/桌面/Web 最终构建均含同摘要 PNG；提取脚本二次运行得到同一摘要。

工作台截图见 evidence/focalroom-brand/branded-workspace.png 与 branded-web-1440.png，三背景 QA 见 cutout-backgrounds.png。90 张历史截图按本轮起始备份恢复并核对摘要（7 张被测试改写后恢复）。本轮没有重跑完整 Rust/后端 E2E、真实模型或原生窗口交互，不将此前基线计数冒充本轮验证。

资源摘要、提取参数和原生包核验记录见 evidence/focalroom-brand/asset-validation.json、extraction.json、native-package.json。原图 SHA-256 为 91c84b24305a32ad2f3beaf80fc985ab97ab5e4ff887cfef027c5623802aa414；最终透明图 SHA-256 为 6f78e666faa81bed6c1bb5c5b2615317bc1f06f3a02a5a87de9ed845f0c8709b。

## 复现与剩余边界

提取脚本为 tools/branding/extract-focalroom-mark.py，仅接受已核验原图摘要，使用本机已有 Pillow、NumPy、SciPy；不是运行或打包依赖，没有新增应用依赖：

```bash
python3 "tools/branding/extract-focalroom-mark.py" "/Users/yihua/Downloads/logo.png" "public/focalroom-mark.png"
```

品牌接入与透明图案已完成本地工程验证；用户尚未评审最终视觉效果。原生 Dock/Finder 的实际显示未做交互验收，签名/公证和发布仍未执行。M3 及后续产品计划的既有剩余项不因改名而关闭。
