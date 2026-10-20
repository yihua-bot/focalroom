# FocalRoom v3.0 · Comprehensive Acceptance Matrix

验收标签：**AUTOMATED**（自动化）、**DEVICE**（实体设备）、**HUMAN**（摄影师/编辑）、**SUPPLIER**（印厂/微喷/外部软件）、**PROD**（生产）。每项必须有实际日期、源码 SHA、环境、结果、证据链接，不允许以“期望通过”填写 pass。

| ID | Gate | Class | 验收场景 | 通过条件 |
|---|---|---|---|---|
| A001 | G0 | AUTOMATED | 最新主分支 CI | typecheck、lint、Rust、API、Web/E2E、迁移均可复现通过 |
| A002 | G0 | AUTOMATED | 旧项目兼容 | 现存 Photo/Sequence/Version/Comment 数据迁移后无孤儿，hash 一致 |
| A003 | G1 | AUTOMATED | 500 张原文件一致性 | 所有 Source SHA256 导入前后相同，未写母版 |
| A004 | G1 | AUTOMATED | 同名不同源文件 | 文件名相同但 hash/asset 不同时不误合并 |
| A005 | G1 | AUTOMATED | 同一资产不同编辑变体 | variant A/B 互不覆盖，版面可锁定旧版本 |
| A006 | G1 | AUTOMATED | 切换编辑软件 | 更改外部链接不改变 assetId 或历史评论 |
| A007 | G1 | AUTOMATED | 丢失和恢复路径 | 失联可见、重新定位 hash 验证成功，不删除 Node |
| C001 | G2 | DEVICE | 16-bit Adobe RGB TIFF 色彩 | reference ICC 变换与校准样本误差处于预定容差 |
| C002 | G2 | DEVICE | ProPhoto RGB 与 P3 显示 | 超色域警告/转换准确，Canvas 与 Native Viewer 匹配 |
| C003 | G2 | DEVICE | ICC 缺失和无效 | 明确警告、绝不显示 ICC Managed 成功 badge |
| C004 | G2 | DEVICE | 多显示器移动 | 目标 ICC 重建、画面无明显错误、profile provenance 更新 |
| C005 | G2 | DEVICE | 100MP 16-bit 巨幅图片 | Fit/100% 瓦片，连续 15 分钟不卡死、不 OOM |
| C006 | G2 | DEVICE | RAW vs embedded JPEG | UI 明确指出代理，不把内嵌 JPEG 叫原始像素 |
| C007 | G2 | DEVICE | 同一图片三视图 | Native Viewer/Canvas/Book Preview 渲染意图一致、像素对照通过 |
| C008 | G2 | DEVICE | 扫描件/灰阶 ICC | 黑白中性、暗部层次、扫描 profile 得到可信结果 |
| C009 | G2 | AUTOMATED | 缓存键完整性 | ICC/decoder/rendition/soft-proof 变更时正确失效 |
| C010 | G2 | DEVICE | HDR/EDR 对照 | HDR 与 SDR 独立标示，不擅自抬高普通照片亮度 |
| N001 | G3 | AUTOMATED | Board 空间坐标 | 指针缩放锚定精度、画布坐标不随 DPR 和滚动改变 |
| N002 | G3 | AUTOMATED | 拖拽取消 | Esc 后 Node 坐标和历史不变；提交单独 undo group |
| N003 | G3 | AUTOMATED | 100+ 图片 Board | 选择/分组/拖拽/缩放与 culling 正确 |
| N004 | G3 | AUTOMATED | 画笔/触控笔 | 压力/矢量 stroke 存储，缩放后无像素漂移 |
| N005 | G3 | AUTOMATED | 评论锚点 | Node 移动/图片局部裁切后 pin 准确；删除 Node 留评论历史 |
| N006 | G3 | HUMAN | 视觉叙事自由探索 | 摄影师建立 2 个 Board、各包含关系分组及便签，不需绕回文件管理器 |
| N007 | G3 | AUTOMATED | Canvas ↔ Sequence | 转换后对象与历史不丢、不意外改变原始 Board |
| AI01 | G4 | AUTOMATED | Tool scope 防越权 | 其他 project/asset ID 的调用硬拒绝、零数据外发 |
| AI02 | G4 | AUTOMATED | Draft-only 改写 | Agent 无法直接写正式 Board/Edition/审批 |
| AI03 | G4 | AUTOMATED | 证据压缩 | 不同照片同名 inspect_assets 工具输出不会错误覆盖重要观察 |
| AI04 | G4 | AUTOMATED | Model route | 能力、区域、授权、摄影质量和费用共同筛选；无默许 fallback |
| AI05 | G4 | AUTOMATED | Feedback memory | 人工纠正引起检索改变，Agent 自己的推荐不当人类偏好证据 |
| AI06 | G4 | HUMAN | 30+ 真人项目 | 10+ 摄影师/编辑盲评五维，包含失败分母，不伪造统计显著性 |
| AI07 | G4 | HUMAN | 与同题基线比较 | 相同照片和任务、版本记录、人类基线及可用竞品结果，报告不确定性 |
| AI08 | G4 | AUTOMATED | 拒绝模型上传 | 未授权时任何真实照片/代理出网计数 0 |
| R001 | G5 | SUPPLIER | Lightroom 30 张同步 | Collection 与稳定 asset ID 匹配，源目录不改 |
| R002 | G5 | SUPPLIER | Lightroom 多次发布 | 新生成 Rendition，同名文件不覆旧版，ICC/字节准确 |
| R003 | G5 | SUPPLIER | Capture One Catalog/Session | 实际目标版本 Recipe 回流成功、变体身份保持 |
| R004 | G5 | AUTOMATED | Ambiguous match | 有两候选时暂停自动归属并可手工确认 |
| R005 | G5 | AUTOMATED | 不完整文件/重复 manifest | 等文件写完+hash/manifest 校验，重复幂等 |
| R006 | G5 | AUTOMATED | 已出版物引用 | 调色回流后冻结版仍引用原旧 Rendition |
| S001 | G6 | AUTOMATED | 两人同 Node 移动 | 冲突可见并可恢复，不静默覆盖正式历史 |
| S002 | G6 | AUTOMATED | 断网草案 | 本地标明未提交，重连 ACL/revision 校验后才可合并 |
| S003 | G6 | AUTOMATED | 代理撤权 | 新票据立即拒绝，Web 清空当前授权像素来源 |
| S004 | G6 | HUMAN | 5 组真人协作项目 | 异地评论/审批/版本/画布同时操作，真实参与者反馈记录 |
| S005 | G6 | AUTOMATED | 跨实例 Presence | 接到相同在线人员/节点状态，不重复；断线能按 cursor 恢复 |
| P001 | G7 | AUTOMATED | 32 页摄影书布局 | 跨页、文字、版式保存及恢复；多 Rendition 不错配 |
| P002 | G7 | AUTOMATED | Preflight 低 PPI | 当低于目标合同阈值时明确阻止 press_ready |
| P003 | G7 | AUTOMATED | 印前颜色和字体 | ICC OutputIntent/嵌入字体/出血/裁切/超界等检查完整 |
| P004 | G7 | SUPPLIER | 真实印厂 PDF/X | 独立预检工具及合作印厂验证，完成实体样书 |
| P005 | G7 | SUPPLIER | 艺术微喷 | 16-bit TIFF + 指定 ICC / 分辨率/锐化交付得到接受 |
| P006 | G7 | AUTOMATED | 冻结 Edition 一致性 | 对同一版本同一环境重建输出哈希或可解释差异清单 |
| L001 | G8 | PROD | macOS 公证发行 | 真正用户包签名、公证、Gatekeeper、更新与回滚 |
| L002 | G8 | PROD | 清洁机器全旅程 | 新账户安装→500 资产→Board→调色→书→交付成功 |
| L003 | G8 | PROD | 灾备 | 存储与 DB 可恢复，私有数据访问权限保持一致 |
| L004 | G8 | PROD | 服务费用与隐私 | provider token 使用/实际账单可核，图片授权与服务支付隔离 |

## 对色彩测试绝不能偷换的定义

色彩参考不是显示器截屏肉眼比较：同一 ICC 输入下需比较源解码/目标空间转换数值，并在校准显示设备结合颜色计测；ΔE00 的阈值要由实际测试方法、样本和设备精度共同定，不能在见到结果后为通过临时改宽。灰阶、肤色、极暗区域、超色域、胶片扫描样本分别分组。

## 品质报告最少字段

`Gate, CaseId, CommitSHA, TestDate, Environment, SourceHash, Expected, Observed, PassFail, EvidencePath, Operator, Caveat`。真实摄影师评测须单列授权、素材来源、评委背景/匿名协议、失败 attempt、盲评表和结果置信区间；不可用测试项填 `NOT_TESTED` 而不是 `PASS`。
