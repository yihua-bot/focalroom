# 最终状态 · 2026-10-26

以下为开发期历史，已由PUBLICATION-GROUP-REMOVAL-2026-10-26.md及evidence/publication-group-removal-2026-10-26/verification.json取代。509前端、175库Rust（2Apple资源跳过）、出版27/审稿9/旧画布4共40条不同本地浏览器通过，严格检查和两前端构建通过。原生功能代码与前批harness一致，已核SHA；仅强化Rust测试并重跑。未提交/合入/安装/生产/发布，完整计划继续。

# 出版整组内容移除 · 接续记录

工作区仍是 focalroom-publication-versions，HEAD 1ed5581；主目录只读。上一批图层已完成：PUBLICATION-LAYERS-2026-10-26.md，504前端/175库Rust（2资源跳过）/39不同本地浏览器。当前新增整组移除尚在最终验收，不能套用上一批数目。

已实现：图层中“解除分组”与“移除整组内容…”明确分开；移除冻结当前版本与全部照片/文字成员，显示数量和前后版面对比，取消不写入。Delete/Backspace只在选中组的图层行/页面对象触发确认，输入框不拦截。锁定及含锁定成员组不可移除；隐藏组仍可明确移除；100项上限有解释。嵌套组删除清空组引用，父组有剩余成员则保留。复用原RemoveFrame/RemoveText一次批量命令、原待确认队列和不可变历史，无新原生功能、迁移或依赖。

新增 publicationRemoval.ts/.test.ts、PublicationGroupRemoval.tsx，repository.removeObjects，四语、Studio接线、UI/队列与第27条出版浏览器旅程。原生既有删除回归已强化同请求幂等及有效旧版恢复。

实际失败已保留：
- 首次新UI测试错误使用只在可交互文字上存在的数据属性计数，改为检查实际照片/文字渲染对象。
- 新队列异常输入用例类型注解位置错误，已改。
- 原生恢复后再加同一照片被既有重复保护拒绝，夹具改用另一张照片，不放松保护。
- 首次浏览器真实发现确认窗口预览figure只有max-height，页面内容溢出遮住“返回”。已改按页面比例限制完整宽度，保留首轮trace/截图于publication-group-removal-2026-10-26/first-failure。没有force点击。
当前日志 /tmp/focalroom-group-removal-*.log。第二轮单旅程、全前端、类型/Lint/契约、Rust库/Clippy/fmt正在执行，须读取结果；先冻结代码，不在浏览器期间构建前端。通过后查看1440/1024截图，必要修补，再构建Desktop/Web，串行跑出版27和旧画布4，审稿9可用另一端口并行。保存最终证据摘要与更新剩余清单。

不合入/提交/推送/安装/部署/发版，不碰生产/模型/Keychain/LUMIX/用户照片。完整G0–G8/Focal AI v2/v3.1/A–K仍继续；本切片不能宣布完整H1或最终计划完成。
