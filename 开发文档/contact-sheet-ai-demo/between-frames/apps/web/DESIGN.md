# FocalRoom Web 协作端设计系统

来源：从官网 `focalroom-website/dist/styles.css`（品牌权威）提取，协作端 `shared.css` 与官网共用同一语言。本文件是协作端所有 UI 的实现契约。

## 1. 设计方向（一句话）

「摄影工作台上的纸与墨」：暖纸底、宋体中文大标题、墨青强调、细线分隔——延续官网的安静专业，像一间亮着灯的暗房外的选片桌，而不是通用深色 SaaS。

## 2. Tokens（全部 UI 值必须出自这里）

```css
:root {
  --paper: #f4f3ee;      /* 页面底 */
  --white: #fafaf6;      /* 卡片面 */
  --ink: #202e31;        /* 主文字/墨青黑 */
  --muted: #737a76;      /* 次级文字 */
  --edge: #dedfd6;       /* 细线边框 */
  --edge-strong: #c9d0c6;/* 强调边框 */
  --cyan: #007da2;       /* 品牌青：主按钮/激活/链接 */
  --cyan-ink: #01536e;   /* 青 hover 深色 */
  --soft-cyan: #e2edef;  /* 青 tint 背景 */
  --green: #599a89;      /* 在线/成功点缀 */
  --danger: #b3423a;     /* 错误（纸感红，非亮红） */
  --serif: Georgia, "Songti SC", "STSong", "Noto Serif CJK SC", serif;
  --sans: -apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", sans-serif;
  --mono: "SFMono-Regular", Consolas, monospace;
  --shadow-card: 0 24px 45px -25px rgba(32,46,49,.33), 0 4px 10px rgba(32,46,49,.07);
  --shadow-float: 0 8px 24px rgba(41,58,53,.13);
  --radius: 6px;
}
```

## 3. 字体

| 层 | 字体 | 用法 |
|---|---|---|
| 展示标题 | --serif 400，字距 -1px | h1/h2/卡片标题（「共享评片」等） |
| 正文 | --sans 400，13px/1.8 | 说明、表单 |
| Eyebrow | --mono 500，10px，字距 1.8px，大写或英文 | 区块标签，颜色 --cyan |
| 元信息 | 11px --muted | 时间、计数 |

## 4. 间距与布局

- 容器：`min(1100px, 100% - 64px)` 居中；移动端 `100% - 40px`
- 区块节奏：垂直 56–96px；卡片内 20–28px
- 细线优先：用 `border-top: 1px solid var(--edge)` 分区，不用大色块

## 5. 原语与状态

| 原语 | 构成 | 状态 |
|---|---|---|
| `.fr-card` | --white 底、1px --edge、--radius、--shadow-card | hover 不浮起（非交互） |
| 主按钮 `.fr-primary` | --cyan 底白字、--radius、内边 10×20 | hover --cyan-ink；disabled 淡化 |
| 次按钮 | 透明底、1px --edge-strong、--ink 字 | hover --soft-cyan 底 |
| 文字链接 | --cyan 下划线偏移 2px | hover --cyan-ink |
| 输入框 | --white 底、1px --edge、focus 边框 --cyan + 外圈 1px --cyan | 错误 --danger 边 |
| 头像 | 28px 圆、无图时渐变 #6c5ce7→#fd79a8 首字 | — |
| 错误条 | --danger 12px，前置 ✕ | role=alert |
| 照片框 | 1px --edge + 4px --white 衬边 + 微阴影，像贴在纸上的样片 | 选中 --cyan 边 + 外圈 |

## 6. 动效

- 仅 transform/opacity；时长 160ms ease-out；hover 颜色过渡 120ms
- 无装饰性动画；照片 hover 仅边框色变化

## 7. 无障碍

- 对比：--ink on --paper ≥ 12:1；--muted ≥ 4.5:1；--cyan on --white 4.6:1
- 焦点：2px --cyan outline、偏移 2px，永不移除
- prefers-reduced-motion: 过渡全部关闭

## 8. 已接受债务

- React dev 工具（react-grab/scan/doctor）未接入 apps/web：会改变依赖树与构建产物，收尾期跳过，后续迭代补
- 照片网格仍用既有类名结构，仅换皮未重构布局 DOM
