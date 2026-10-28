# 文字方向与书脊排字接续 · 2026-10-26

2026-10-27 最终接续：本WIP已由PUBLICATION-TYPOGRAPHY-2026-10-27.md和evidence/publication-typography-2026-10-27/verification.json替代。575前端、215库Rust+2既有Apple资源跳过、严格检查/两构建通过；46出版+9审稿完整基线后，退出确认新旅程发现真实视口问题并修复，相关4与旧工作区4通过，共60不同流程。所有本批进程已结束，以下session均为历史，不据此重新启动。主线整合准备的只读76文件盘点已生成；尚未整合、提交、部署或发行，完整目标继续。

2026-10-27 细节预览接续：首轮新2条已经通过，但实际截图文字过小、窄窗按钮不在视口；增加PublicationTextPreview，仍用同一Paper，以100%排字/DOM glyph范围定位文字，支持整页切换、内部滚动与字体/尺寸变化观察；溢出在细节头部显示，文字表单操作固定底部。第二轮2条亦通过，并新增五方向真实字形可见范围、整页切换、100%约13.333px字号及窄窗保存可见断言。已查看second-browser三张图，pending提示原来在视口外，现加文字窗口专用顶部sticky反馈并加可见断言。最新前端575全量/类型/Lint/契约/两构建运行session52604，先查typography-*-final.log；完成后须再验新2和旧字体截断、截图及完整46+9+4。Native未因细节预览改变，215+2/Clippy/fmt/harness有效。56021、15091对应2+2流程已经终止，不按旧状态重启。

2026-10-27接续：本轮新增2条真实本地Store浏览器已经写入，定向文字方向流程运行中，exec session56021，日志/tmp/focalroom-typography-browser-first.log；先查当前进程/结果，不因旧记录重新启动。严格Clippy/fmt、Desktop/Web构建、类型/Lint已通过，harness按当前Native已重建。新增流程计划覆盖窄书脊五种实际computed style/物理框、真实溢出与修正、确认前不写、命名恢复/重启和方向回执冻结。实际截图尚待看；完整出版将为46，审稿9、旧工作区4仍须验。

状态：实现进行中，尚未完成浏览器与最终验收。用户询问最终剩余范围时暂停新增代码、核对完整计划；完整G0–G8/Focal AI v2/v3.1/A–K保持。上一轮展开封面已完成本地切片，不能把这轮文字方向预先列为验收完成。

## 已接代码

- 既有AddText/UpdateText/CreateChapter.title新增可选textDirection，五值horizontal/vertical-rl/vertical-lr/rotate-90/rotate-270。旧缺字段请求继续缺字段，显式null/未知值拒绝。Native Direction在publication_typography.rs，编辑写入时旧UpdateText缺字段保留已经保存的方向，不补默认值。
- 现有styleJson保存方向，无新表迁移依赖。Native版本与TS读取验证不支持方向；损坏当前可显式恢复有效历史，不能冒充横排。
- 复用文字编辑器与Paper，框内预览、缩略图、母版与历史使用同一方向。物理毫米矩形不交换；旋转仅作用于内部排字框。溢出比较内排字框scroll/client尺寸，后续必须在真实浏览器检查竖排/旋转、不同字号、修正后消失和缩放。
- 四语六个方向字段已加，字体仍为既有三种通用字体，并未实现富文本、字体管理/嵌入或生产印前。
- 新源文件：Native publication_typography.rs、publication_typography_tests.rs；TS domain/publicationTypography.ts及test；features/publishing/publicationTextDirection.ts、PublicationTextDirection.test.tsx。修改了既有contracts/publication、TextUpdate、versions、Paper/TextBox/TextEditor、chapter preview、scope读取和持久客户端测试。旧4个Native测试构造器仅加text_direction:None。

## 已有测试证据

- 本轮新测试前，209库Rust+2Apple资源跳过、568旧前端全量通过，证明缺字段兼容基线。
- typography_editing新6项Native通过，211项未选；包含版本/回执/恢复/重启、旧客户端保留、隐藏继承锁/权限/批量回滚、五值/缺字段/null、母版预览/应用、历史晚期失败/损坏恢复。
- TS新增定向3文件13项通过，其中7项为本批新增（domain3、编辑器3、持久客户端1），其余为原客户端回归，不能写成13项新测试。
- 最新全量前端/类型/Lint/契约与Native库测试已发起；日志/tmp/focalroom-typography-front-full-current.log与native-full-current.log，先查现有进程/结果，不因本记录猜测重启。

## 下一步

1. 记录当前全量实际结果，修正真实失败；严格Clippy/fmt及重建harness（当前harness仍为上一批封面代码，不能拿它验新字段）。
2. 加真实本地Store浏览器旅程：窄书脊上的五种方向实际computed style/物理框范围、旋转与竖排文字的真实溢出及修正、明确保存前不写、历史恢复/重启、丢回执只确认原AddText/UpdateText。边界断言按真实状态，不force/sleep。
3. 读实际1440/1024截图，完成构建、回归、证据和正式报告后再更新H1完成状态。出版基线44+新流程，审稿9、旧工作区4；旧工作区需串行同1456/1457及RPC_BASE。
4. 继续富文本/字体、母版图层、跨页混合组、生产Edition/渲染/印前及完整A–K；本地方向支持不代表新数据到旧发行客户端的降级安全，K4仍待核验。

仍在独立工作区，main只读。未提交/推送/合入/生产/模型/Keychain/LUMIX/用户照片/安装/签名/发布。不要重启旧UNKNOWN真实300图任务。
