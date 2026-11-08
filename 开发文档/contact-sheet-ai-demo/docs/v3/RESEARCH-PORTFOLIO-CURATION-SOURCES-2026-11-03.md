# 作品集策展外部方法论调研（依据清单）

日期：2026-11-03，Asia/Shanghai。目的：为 FocalRoom「作品集整理」AI 提示词补强提供有文献依据的方法清单。本文件只登记外部来源与可转写规则，不修改任何提示词或代码。

调研方法说明：本轮环境 web_search 不可用，DuckDuckGo/Bing/Mojeek 反爬、LensCulture/Magnum/PetaPixel/MoMA 直接抓取被拒。实际采用三条路线：① 主线直接抓取已知来源 URL（Aperture 站内一手材料）；② 子任务 A 以「已知站点探测 + WordPress REST API」获得 A Photo Editor 的实践者长文；③ 子任务 B 以「curl 直抓 + sitemap 定位」获得六个机构的明文页面。所有引用均来自 2026-11-03 实际抓取的页面/PDF 原文（子任务来源经父会话独立复抓核验），未抓到的材料在文末「局限」中如实列出。

---

## 一、来源清单（按证据等级）

### A. 一手教学材料（Aperture On Sight 课程，K-12 摄影书教学）

| 编号 | 来源 | 形式 |
|---|---|---|
| S1 | Aperture On Sight Lesson 15「Extending the Project」 https://aperture.org/on-sight/lesson-15/ | 课程页 |
| S2 | Aperture On Sight Lesson 16「Gordon Parks and The Making of an Argument」 https://aperture.org/on-sight/lesson-16/ | 课程页 |
| S3 | Aperture On Sight Lesson 17 教案 PDF「Book Dummies and Meeting an Editor」 https://aperturewp.wpengine.com/wp-content/uploads/2020/05/2017_02_27_ApertureOnSight_LessonPlan_17_SL.pdf | 课程教案（1页） |
| S4 | The Arrangement Game 练习 PDF（Lesson 17 课堂资源） https://aperturewp.wpengine.com/wp-content/uploads/2020/05/2017_02_13_ArrangementGame_Lesson17_Small.pdf | 排序练习说明（6页） |
| S5 | Aperture On Sight Lesson 17 课程页 https://aperture.org/on-sight/lesson-17/ | 课程页 |

### B. 权威机构征件标准与申请指南（作品集评审的一手标准）

| 编号 | 来源 | 形式 |
|---|---|---|
| S6 | Aperture Portfolio Prize 2026 征件页（含评审标准原文） https://aperture.org/calls-for-entry/portfolio-prize/ | 官方征件页 |
| S11 | Photolucida Critical Mass 2026 提交细则与 FAQ https://www.photolucida.org/critical-mass/entry-details/ 与 https://www.photolucida.org/critical-mass/faq/ | 官方征件页 |
| S12 | SAIC 摄影 MFA 招生页 https://www.saic.edu/photography/graduate/mfa-studio-admissions-information | 院校官方指南 |
| S13 | RISD 摄影 MFA / 研究生申请页 https://www.risd.edu/academics/photography/masters-program | 院校官方指南 |
| S14 | Goldsmiths MA Photography: Urban Cultures https://www.gold.ac.uk/pg/ma-photography-urban-cultures/ | 院校官方指南 |
| S15 | photo-eye Gallery（Santa Fe）提交页 https://www.photoeye.com/gallery/submissions | 画廊官方指南 |
| S16 | FotoFest Meeting Place（Houston） https://fotofest.org/meeting-place | 评审活动官方页 |

### C. 编辑与学者的公开论述

| 编号 | 来源 | 形式 |
|---|---|---|
| S7 | David Campany「What Does Photo Editing Look Like?」，Aperture，2018-12-20 https://aperture.org/editorial/campany-photo-editing/ | 长文（The PhotoBook Review 期文） |
| S17 | Tony Fouhse「Anatomy of an Edit: Making a Photobook」，A Photo Editor，2023-04-19 https://www.aphotoeditor.com/2023/04/19/03-anatomy-of-an-edit-making-a-photobook-with-tony-fouhse/ | 实践者第一人称方法长文（8 部曲 Ep.3，源自其 newsletter HYPO） |

### D. 百科/条目确认（背景性，方法密度低）

| 编号 | 来源 | 确认内容 |
|---|---|---|
| S8 | Wikipedia「The Americans (photography)」 https://en.wikipedia.org/wiki/The_Americans_(photography) | Robert Frank 的书被描述为「sequenced poetically, conceived without text like a silent movie」的诗意排序范本；版本演变史 |
| S9 | Wikipedia「John G. Morris」 https://en.wikipedia.org/wiki/John_G._Morris | LIFE 图片编辑，回忆录《Get the Picture》（Campany 也点名：伟大图片编辑极少留下方法论） |
| S10 | Wikipedia「Gerry Badger」/「Photobooks」 https://en.wikipedia.org/wiki/Gerry_Badger | 确认《The Photobook: A History》三卷本（与 Martin Parr 合著）存在及其地位；卷内具体论点未获取 |

---

## 二、按缺口维度的方法发现

### 1. 编辑流程的操作步骤（S3/S4，可直接转写）

**核心流程：先配对，再把配对组成顺序。**
> "start making sequences by putting the photographs in pairs, then organizing the pairs into a thoughtful order"（S3）

**Arrangement Game 的完整操作（S4）**：
1. 选 8–10 张进入一个排列（练习规模；正式书 dummy 物料为 25–30 张快印 + 约 20 页书 blank，S3）
2. 排序的候选起点维度：① 讲故事；② 按色调分组；③ 按类型分组（肖像/物件/风景）
3. 「质疑你的直觉。既做明显的配对，也做意想不到的配对。思考它们如何互相『说话』」
4. 形态可以是一条线，也可以是金字塔（非线性排列）
5. **完成后拍下来，洗牌，重新开始**——多方案产出是方法的一部分，不是可选项

**对默认叙事的批判（S3）**：
> "The most obvious sequence is often chronological or one that imposes a narrative on a series of photos. But is that really what the photo story is about?"
（最显而易见的序列是时间顺序或强加叙事——要质疑这是否真是这组照片的内容。）

### 2. 节奏与多样性（S3，可直接转写）

- **距离多样性**：序列中要包含「close-up, medium, wide」的交替（特写/中景/远景）——但提醒包含，不强制逐张交替。
- **节奏定义**："Think about rhythm: like songs, photo series can have calm sections and more energetic sections. Some photographs are quiet and still, some are full of action."（像歌曲一样有平静段与能量段；有的照片安静静止，有的充满动作。）

### 3. 「代表性」与作品集质量门槛（S6，权威评审标准原文）

Aperture Portfolio Prize 提交规格：**10–15 张**（JPEG 3000px 长边）+ 作品陈述。评审五条标准（原文翻译）：

1. 作品是否创作于近五年内？（时间范围约束）
2. **主题或议题是否以「完全解决、一致」的方式贯穿整个作品集？**（主题一致性——「宁缺毋滥」的机构版本）
3. **每张照片是否单独站得住？**（单张强度与序列质量并重）
4. 技术/呈现选择是否服务于作品（如黑白 vs 数码增强调色板须有明确理由）？
5. 对题材是否采取了创新或独特的切入？

附加原则："It is the portfolio that counts."（只看作品集本身，不看摄影师地位。）

→ 这五条可直接改写为 AI 草案自评清单的骨架。

### 4. 项目完整性与主题演进（S1，可直接转写）

Lesson 15 的方法：清点已有照片 → 制作「shot list」列出**还缺哪些照片**才能完成项目 → 接受「主题会变、常需补拍」。
→ 对应 AI 策展的「缺口分析」步骤：不是只从已有照片里挑，还要指出序列缺什么类型的照片。

### 5. 语境、受众与编辑判断（S2，可直接转写）

Lesson 16 用 Gordon Parks 首个 LIFE 摄影随笔做案例：同一批照片，**排序与语境不同则意义不同**；学生「基于想让照片讲什么故事」做编辑选择，并「按照片与所选主题的契合度来编辑和排序」。
→ 编辑决策的依据是主题契合度，不是单张分数；且要显式回答「想让这组照片讲什么」。

### 6. 理论边界（S7，防止把方法夸大成理论）

Campany 文章的核心论点（直接影响提示词该用什么口吻）：

- **照片组合更像诗歌而非散文**："Most combinations of photographs are more like poetry than prose... image combinations *suggest*"——组合暗示，不解释，不擅长论证。
- **单张与整体的永恒张力**："There is always a tension... between what is irreducibly singular about a photograph and the part it plays in the larger whole."——这正是 FocalRoom「单张强度 + 序列作用」双重标准的理论表述。
- **没有普适理论**："There can be no overarching theory... the more specific one can get, the better."——不存在图像排列的大理论，方法越具体越好。
- **摄影没有蒙太奇理论**：电影有苏联蒙太奇理论（Eisenstein/Kuleshov/Vertov），摄影从未发展出对应理论；伟大图片编辑（Lorant、John G. Morris、Delpire）极少著述方法。
- **Keith Smith《Structure of the Visual Book》(1984)**："one of the few sustained attempts to think through the relations between images"——思考图像间关系的少数专著（原书内容未获取，仅按 Campany 引述登记）。
- **书的地位**：对当代摄影师，书是「最可能全面表达艺术视野的空间」。
- **当代已无主流编辑惯例**：主导数十年的大众杂志图片故事惯例已瓦解，"Today there is no fixed order to kick against"——没有固定顺序可供反叛，各种排列都在被尝试。
- **锁定序列是一种抵抗**：在图像顺序日益由「意识形态、偏好、品味与商业的算法」决定的时代，一次性锁定一个序列"can seem like a small act of resistance"。（注：此论点可作产品哲学注脚——人工编辑决策对抗算法流——但属于评论性观点，不是操作方法。）

→ 与 FocalRoom 现有立场（札记自我声明「不是学术共识」「方法只是可试验的编辑假设」）高度一致；提示词应保持「具体可操作步骤」而非「理论体系」的写法。

### 7. 作品集类型学与张数基准（S6/S11–S16，全部为机构明文）

**按用途的张数区间（机构硬性规定）**：

| 用途 | 张数规定 | 来源 |
|---|---|---|
| 艺术评审制比赛 | **7–10 张硬区间**，少于 7 张直接取消资格（Photolucida Critical Mass） | S11 |
| 艺术比赛（Aperture Prize） | **10–15 张** | S6 |
| 艺术院校申请 | **15–20 张**静态图，或 ≤10 分钟时间媒介，或混合（1 张图 ≈ 2 分钟视频）（SAIC 摄影 MFA） | S12 |
| 院校申请（RISD 总则） | studio 类统一 **10–20 件**视觉作品 | S13 |
| 画廊线下 | 实体作品集**最多 20 张**（photo-eye Gallery）；呈现单元是「Portfolio/Project」而非散片 | S15 |

**结构与陈述规则**：

- **默认单一系列**："Submit only one body of work"，每年限投 1 个项目（S11）；多系列混排仅在明文允许时出现
- **陈述绑定系列本身**：艺术家陈述 ≤250 词，且「应针对所提交的该组具体作品，而非整体艺术哲学」（S11）
- **深度/广度比例有先例**：RISD MAT 要求 20 张 = 10 张单媒介深挖 + 7 张多媒介 + 3 张素描（S13）——「单一媒介内深挖 + 少量广度」的结构化拆分
- **edited 是最低要求**：Goldsmiths 唯一强调点是「An edited portfolio of visual images」——经过编辑精选，未规定张数（S14）
- **时效不是判据**：「作品无需是近年的，任何时期均可」（S11）——代表性 > 新近性
- **复投换血**：入选 Top 200 者复投最多复用原提交中的 2 张（S11）——强制引入新作品
- **评审注意力单元**：FotoFest Meeting Place 每场一对一是 **20 分钟 × 单一系列**；官方定位区分两种目的——「完成度高的完整作品集寻求实际结果」vs「进行中作品获取反馈」（S16）

### 8. 一个完整的编辑实操案例（S17，实践者第一人称）

Tony Fouhse（加拿大摄影师，前全职图片编辑）公开的完整方法，含量化数据：

1. **快速初剪**：每次只花 10–15 分钟做快照式决策，不存早期迭代；一天多次短时接触优于一次长时间
2. **发现式编辑**：初剪检验「照片是否支持初始构想」；若不支持，让照片揭示实际拍到的主题（作者案例：预设「摇篮曲」→ 照片揭示实为「童话」），以新前提重排
3. **量化漏斗**：项目全程 **987 张 → 初选文件夹 152 张 → 最终序列 46 张**（≈4.7% 入选率）——可作 AI 策展的参数基准
4. **虚拟→实物验证环**：小尺寸工作样片装订成 dummy 随身携带，向信任者征求「批评而非夸奖」；明确预期「这只是第一版，还会有很多版」
5. **序列难度分级**：均质序列（同批人像/风景）排序容易且重要性低；异质图像需复杂「编织」（weaving），排序是项目成败关键——可映射为组内同质性检测维度
6. 工具经验：Blurb Bookwright（免费）专用于初排（快速换序/拖页），终稿另用本地印刷

---

## 三、与 FocalRoom 现有 8 篇方法札记的对照

| 札记 | 外部对应 | 对应强度 |
|---|---|---|
| K001 观看距离与叙事节奏 | S3 距离多样性 + 节奏（平静/能量段） | 强：几乎同构 |
| K002 三种创作线索 | S4 排序起点维度（故事/色调/类型）；S17 发现式编辑（照片推翻预设主题后按新线索重排） | 强：精神一致（多起点找实质差异），维度清单互补（K002 为空间/人物/时间动作，S4 为故事/色调/类型）；「每条线索先写一句可被照片验证的问题」是札记比外部来源更细的一步 |
| K003 视觉事实与编辑判断 | S7 事实 vs 暗示；S2 主题契合度判断；S17「照片是否支持初始构想」校验 | 强（升）：外部三方呼应「用照片校验构想」 |
| K004 色彩与版本比较 | 无直接外部来源抓到 | 弱：待补 |
| K005 摄影书初稿 | S3 全套（物料规模、配对→顺序、dummy、同伴反馈）；S17 完整案例（含工具链） | 强 |
| K006 从反馈提炼偏好 | S3 同伴反馈/gallery walk（Post-it 反馈）；S17 dummy 反馈环（「批评而非夸奖」、多版本预期） | 中：外部来源到「要结构化批评」，仍未到「偏好沉淀」 |
| K007 重复照片的取舍 | 无直接外部来源抓到 | 弱：待补 |
| K008 先保留原版 | S4 洗牌重排、S3 书 blank 誊录前先解释理由；S17「不存早期迭代」是对立面——注意两者张力：FocalRoom 保留全部父版，Fouhse 初剪阶段刻意丢弃 | 中：精神一致，阶段不同（初剪 vs 定稿迭代） |

未覆盖外部对应的札记（K004/K007）与仍未覆盖外部来源的缺口（商业委托作品集、年度回顾选片）见「局限」。

---

## 四、可转写为提示词规则的候选清单

按来源强度排序，每条可直接改写进「作品集策展」提示词或自评清单：

**流程类**：
1. 先配对、再把配对组成顺序（S3）
2. 产出多个替代方案是必经步骤：完成后洗牌重排，方案间需有实质维度差异（S4；配合 K002）
3. 质疑时间顺序/强加叙事的默认排列，显式回答「想让这组照片讲什么」（S3/S2）
4. 发现式编辑：用照片校验初始构想，照片不支持时让照片揭示实际主题、以新前提重排，而不是硬塞进预设（S17）
5. 初剪与定稿分离：初剪快速多轮（参考 10–15 分钟短时多轮），定稿阶段才保留全部版本供比较（S17 + FocalRoom K008 的阶段化调和）
6. 不仅挑选已有照片，还输出「缺口清单」——序列还缺什么类型的照片（S1 shot list）

**多样性与结构类**：
7. 检查序列中特写/中景/远景的分布；检查安静照片与高能量照片的分布（S3）
8. 序列同质性检测：均质序列排序容易但作用有限；异质序列需要显式「编织」，排序策略随同质性变化（S17）
9. 组合意义允许「暗示、留白、张力」，不要求逻辑论证（S7 诗歌 vs 散文）

**规模与类型类**：
10. 张数缺省区间按用途选择：比赛型 7–10，奖项型 10–15，申请型 15–20（SAIC）/10–20（RISD），画廊 20 封顶（S6/S11–S13/S15）
11. 默认单一 body of work；多系列混排仅在用户明确要求时使用（S11/S15）
12. 时效不是选片判据——「任何时期作品均合格」有机构明文（S11）
13. 作品集更新时限制旧作复用（Photolucida 复投 ≤2 张旧图的先例），鼓励换血（S11）

**质量门槛类**：
14. 自评五条：主题以「完全解决、一致」方式贯穿；每张单独站得住；技术选择有理由；题材切入独特；时间范围明确（S6）
15. 取舍依据是「与所选主题的契合度」，不是单张分数（S2）
16. 漏斗参考：全量素材 → 初选（~15%）→ 最终序列（~5%）的收敛率可作 AI 预算与目标张数的基准（S17）
17. 对外陈述绑定该系列本身（≤250 词参照），不写总体艺术观（S11）
18. 每个方案附编排理由说明——「能描述自己的编辑过程」是教学目标的显式要求（S3）

**口吻与边界类**：
19. 方法表述保持「具体步骤与可试验假设」，不冒充理论体系（S7：无普适编辑理论，越具体越好）
20. 方案定位二选一：完整作品集求结果，还是进行中作品求反馈——两种目的的完成度标准不同（S16）

---

## 五、局限与未获取材料

以下材料本轮确认存在但未能获取原文，引用时须注明「未核验」：

- Keith Smith《Structure of the Visual Book》(1984) 原书内容——仅有 Campany 转述（S7）
- Parr & Badger《The Photobook: A History》三卷本卷内论点——仅确认地位（S10；维基条目无论点概述，Phaidon 书页/eye 杂志书评未测）
- Magnum 编辑方法（magnumphotos.com 整站 403）、Alex Webb / Rebecca Norris Webb 夫妇 workshop 方法（域名不可达；替代路径：其书《On Street Photography and the Poetic Image》Aperture 版书页或 Wayback 抓 Magnum Learn）
- 商业/编辑委托类作品集逻辑（Format magazine 429/404，AOP/ASMP 未及验证）
- 年度回顾/best-of 选片文章（PetaPixel/Shotkit 403/404）
- 「best work vs representative work」的评论/教学文章——全部被拒，本清单中「代表性」以 S6/S11 机构标准替代支撑
- SVA（各 URL 变体 404）、Yale、Parsons（429）、Arles experts 页（JS 无正文）、LensCulture /submit（403）、Light Work（404）

另：S1–S5 为 K-12 教学课程，其规则经过课堂简化，转写为产品提示词时应保留其操作结构、按成年创作者场景调整表述。
