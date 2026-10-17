# M3 持久化增量句向量索引验收

日期：2026-10-17（Asia/Shanghai）。接续 [上一切片](V2-M3-LOCAL-SEMANTIC-TEST-REPORT.md)；设计见 [ADR 0036](ADR/0036-m3-persistent-semantic-index.md)。M3整体仍 partial。

## 功能结果

- `search_photos_semantic` 可跨数据库连接重开复用向量。中文/英文真实系统模型分别为revision1/640维、revision1/512维；冷启动3张事实生成3条文档向量，重开后范围内1张命中1条、生成0条；只改1张时命中2条、生成1条。查询向量始终重新推断，不能把生成0条文档说成零模型调用。
- 读取时仍先按任务scope、当前源hash和来源取观察；模型语言/revision/维数及完整观察摘要必须匹配。结果保留引用与覆盖限制；相似度不是置信度或审美评分。文件只含摘要与私密派生向量，不保存原文/查询/原图，不外发。
- 写事务重验通过才发布当前候选，按照片替换旧版本，最多500条/8MiB。发布合并最新文件；checksum/版本/有限数值/长度校验拒绝损坏数据。原子替换、Unix0600、已知链接拒绝；文件写失败明确标注，不影响有效字面/语义结果。
- 既有清空按钮现在说明并清除相关检索缓存；文件清空失败时观察和命令回执保持原样。成功回执幂等，旧预取不能在清空后重新发布已失效向量。正式选片、序列、证据和源文件不变。

## 验证记录

最终日志、源码摘要、构建元数据见 `evidence/v2-m3-persistent-index/`；通过数以归档summary为准。默认新增5项文件/命令边界测试，增强既有过期预取测试；另新增1项显式原生复用测试，并扩展既有中文/英文Agent工具测试。

最终各项检查通过：**165项桌面Rust、13项云测试、169项Vitest、16项预览E2E、隔离重跑6项真实后端E2E**；契约、类型、Lint、fmt及严格Clippy通过。显式原生lib测试2项、真实语义工具测试1项通过；过期预取增强断言额外定向复验1项通过，不重复计入165。默认13项ignored，其中本轮显式执行上述3项；钥匙串写删1项filtered，未把未执行项算通过。

全量脚本首轮exit=1：最后的提案界面测试等待面板超时，其他5条后端旅程及前面的全部检查通过。trace记录点击后约3秒再次出现Vite连接和React初始化，页面整页刷新将AI面板重置为照片面板；当时并行的native打包会生成前端产物，干扰了开发服务器验收。打包结束后**顺序重跑整个后端套件exit=0、6/6通过（1.2分钟，提案6.6秒）**，没有改产品逻辑、加点击重试或延长超时。首轮日志与Vite事件仍保留，不把首轮exit码改写成0。今后原生打包与使用Vite的界面测试须顺序运行。

独立debug包构建通过（58.36MiB，`studio.betweenframes.m1reviewfixes`），NaturalLanguage弱链接检查通过；[二进制元数据](evidence/v2-m3-persistent-index/build-metadata.json)已归档，未启动原生窗口或用户库。最终253个源码摘要一致；全量运行中只增强了photo_search.rs的cfg(test)缓存失效断言，已额外定向补验，生产逻辑无变化。90张起始截图全部字节一致，恢复了25张本轮测试产物；不是checkout到HEAD，保留开始时已有工作区修改。

| 场景 | 断言 |
|---|---|
| 文件重开/增量替换 | 向量回读一致；同照片旧键移除，其他候选保留；501条淘汰至500条；版本变化清旧配置 |
| 损坏与资源边界 | checksum不符、超8MiB、非法维数、重复键、NaN拒绝；无效写入不破坏原有效文件，后续可重建 |
| 隔离与链接 | 不同数据库/项目不同文件；目录/文件符号链接拒绝，目标哨兵字节不变；Unix文件0600 |
| 清空与幂等 | 清空后有效空索引；重放不删除后续新缓存；失败时观察仍在且零命令回执；正式状态/证据不变 |
| 原生模型 | 真实query/vector；命中后文档生成0，language/revision/dimensions错配生成1而非复用 |
| Agent工具 | 中英文冷启动/连接重开/范围/单张更新/清空后无结果；零上传、零费用、零偏好证据、原图SHA/正式项目一致 |
| 失效预取 | 异调用、移出scope、观察正文变化/删除、取消拒绝；清空后旧pending向量不可重新落盘 |

原生第一次增量测试失败保留：重复英文事实被系统语言识别为nl，因此未生成向量。另行调用本机API确认“语言nl但英文模型可生成512维”；工具按现有同语言规则明确跳过、limited=true，不降低断言。换成系统识别为en的单句事实后，两个语言的增量断言均通过。这是已知识别/召回限制，不是摄影质量结论。

## 复现

在 `between-frames/`：

```sh
bash "scripts/verify-all.sh" --skip-keychain
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --lib native_ -- --ignored --nocapture
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --test photo_search semantic_tool_finds_paraphrases_with_real_local_embeddings_and_cited_scope -- --ignored --exact --nocapture
```

第二条仅选中lib内两个原生句向量测试；没有批量运行真实API/真实照片ignored测试。默认钥匙串写删仍跳过。本轮无DeepSeek请求、新照片/真人评测、依赖/迁移、线上数据库操作、部署、SSH或Git提交推送；原用户数据库未打开。继承工作区已有改动，保留原始截图和旧验收记录。

## 剩余问题

本轮完成现有缓存事实的持久化增量推断；仍不是全图库视觉索引、ANN或低价观察/高能力策展分工。每次读取/重写有界完整文件，SQL聚合随观察历史增长；3秒为协作式推断预算，不是总耗时硬实时限制。每项目单一模型配置，语言切换会重建；没有全局磁盘限额、后台填充或崩溃临时文件清理。

旧派生条目可能留到淘汰/主动清空；回执、备份、文件系统旧块和内存不保证安全擦除。向量敏感且未加密，校验和防损坏而非本机恶意修改认证。系统语言识别和源事实均可能出错，专业召回率/延迟/质量还需评测。

后续先补M3索引/策展模型分工与受控观察策略、作者记忆Agent接线、完整供应商授权执行，再补真实任务与质量评测；M4/M5生产身份/Postgres/事务/实时同步门槛保持单列。DeepSeek新Prompt真实回归仍待可用配置，本轮没有把历史记录计作新通过。详见 [剩余计划](V2-REMAINING-PLAN.md)。
