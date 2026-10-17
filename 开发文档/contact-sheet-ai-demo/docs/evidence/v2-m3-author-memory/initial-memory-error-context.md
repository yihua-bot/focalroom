# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: memory-controls.spec.ts >> M4 记忆控制：确认、反例、排除/恢复、移除/恢复、搜索及重启
- Location: e2e/memory-controls.spec.ts:14:1

# Error details

```
Error: locator.check: Clicking the checkbox did not change its state
Call log:
  - waiting for getByRole('checkbox', { name: '允许离线 Agent 引用其他本地项目记忆' })
    - locator resolved to <input type="checkbox" aria-label="允许离线 Agent 引用其他本地项目记忆"/>
  - attempting click action
    - waiting for element to be visible, enabled and stable
    - element is visible, enabled and stable
    - scrolling into view if needed
    - done scrolling
    - performing click action
    - click action done
    - waiting for scheduled navigations to finish
    - navigations have finished

```

# Page snapshot

```yaml
- generic [ref=f1e3]:
  - complementary [ref=f1e4]:
    - generic [ref=f1e5]:
      - generic [ref=f1e6]: B
      - generic [ref=f1e7]:
        - generic [ref=f1e8]: BETWEEN
        - generic [ref=f1e9]: FRAMES · STUDIO
    - button "收起导航" [ref=f1e10] [cursor=pointer]
    - generic [ref=f1e14]: WORKSPACE
    - navigation "主导航" [ref=f1e15]:
      - button "选片工作台" [ref=f1e16] [cursor=pointer]
      - button "作品序列" [ref=f1e20] [cursor=pointer]
      - button "审美记忆" [ref=f1e25] [cursor=pointer]
      - button "项目列表" [ref=f1e30] [cursor=pointer]
    - generic [ref=f1e35]: CURRENT PROJECT
    - button [ref=f1e36] [cursor=pointer]:
      - strong [ref=f1e37]: M4 · 选择有证据
      - generic [ref=f1e38]: 199 张图像 · 0 张入选本地项目
    - generic [ref=f1e39]:
      - button "快捷键帮助" [ref=f1e40] [cursor=pointer]
      - text: 本地保存 · 原图只读完全离线选片 · 无云端上传
  - main [ref=f1e44]:
    - generic [ref=f1e45]:
      - generic [ref=f1e46]: PROJECT / NEW — EDITING ROOM
      - generic [ref=f1e47]:
        - generic [ref=f1e48]:
          - heading "M4 · 选择有证据" [level=1] [ref=f1e49]
          - paragraph [ref=f1e50]: 从照片，到作品。 / 让每一次选择都有来处。
        - generic [ref=f1e51]:
          - button "导入照片" [ref=f1e52] [cursor=pointer]
          - button "Agent · 本地 Mock" [ref=f1e54] [cursor=pointer]
      - generic [ref=f1e57]:
        - generic [ref=f1e58]: “
        - paragraph [ref=f1e59]: 人工证据可排除、假设可纠正与恢复。
        - button "编辑创作意图 ↗" [ref=f1e60] [cursor=pointer]
    - group "编辑历史" [ref=f1e61]:
      - button "撤销" [ref=f1e62] [cursor=pointer]
      - button "重做" [disabled] [ref=f1e63]
      - button "作品版本" [ref=f1e64] [cursor=pointer]
      - generic [ref=f1e65]: 4 次可撤销编辑 · 本地保存
    - generic [ref=f1e66]:
      - generic "照片筛选" [ref=f1e67]:
        - button "全部 199" [pressed] [ref=f1e68] [cursor=pointer]:
          - text: 全部
          - generic [ref=f1e69]: "199"
        - button "已保留 0" [ref=f1e70] [cursor=pointer]:
          - text: 已保留
          - generic [ref=f1e71]: "0"
        - button "待定 198" [ref=f1e72] [cursor=pointer]:
          - text: 待定
          - generic [ref=f1e73]: "198"
        - button "已淘汰 1" [ref=f1e74] [cursor=pointer]:
          - text: 已淘汰
          - generic [ref=f1e75]: "1"
      - generic [ref=f1e76]:
        - button "切换网格密度" [ref=f1e77] [cursor=pointer]
        - button "两图对比" [ref=f1e80] [cursor=pointer]
        - button "导出 ↗" [ref=f1e81] [cursor=pointer]
        - button "收起序列带" [pressed] [ref=f1e82] [cursor=pointer]
        - button "收起检查器" [ref=f1e85] [cursor=pointer]
      - generic [ref=f1e89]:
        - searchbox "搜索照片文件名或备注" [ref=f1e94]
        - generic [ref=f1e95]: ⌘ / Ctrl 点选 · Shift 连选
    - generic [ref=f1e96]:
      - generic [ref=f1e97]:
        - generic [ref=f1e98]:
          - generic [ref=f1e99]: CONTACT SHEET
          - generic [ref=f1e100]: 199 FRAMES · 完整构图 · K / X 快捷操作
        - generic [ref=f1e101]: 本地项目 · 所有已保存的判断和顺序将在重启后保留
        - generic "照片网格" [ref=f1e102]:
          - generic [ref=f1e103]:
            - article [ref=f1e104]:
              - button "查看第 1 张 frame-001.jpg" [ref=f1e105] [cursor=pointer]:
                - img "frame-001.jpg" [ref=f1e106]
                - generic [ref=f1e107]: "01"
              - generic [ref=f1e108]:
                - generic [ref=f1e109]: FRAME 01
                - generic [ref=f1e110]:
                  - button "标记保留 frame-001.jpg" [ref=f1e111] [cursor=pointer]
                  - button "标记淘汰 frame-001.jpg" [ref=f1e114] [cursor=pointer]
            - article [ref=f1e118]:
              - button "查看第 2 张 frame-002.jpg" [ref=f1e119] [cursor=pointer]:
                - img "frame-002.jpg" [ref=f1e120]
                - generic [ref=f1e121]: "02"
              - generic [ref=f1e122]:
                - generic [ref=f1e123]: FRAME 02
                - generic [ref=f1e124]:
                  - button "标记保留 frame-002.jpg" [ref=f1e125] [cursor=pointer]
                  - button "标记淘汰 frame-002.jpg" [ref=f1e128] [cursor=pointer]
            - article [ref=f1e132]:
              - button "查看第 3 张 frame-003.jpg" [ref=f1e133] [cursor=pointer]:
                - img "frame-003.jpg" [ref=f1e134]
                - generic [ref=f1e135]: "03"
                - generic [ref=f1e136]: × 已淘汰
              - generic [ref=f1e137]:
                - generic [ref=f1e138]: FRAME 03
                - generic [ref=f1e139]:
                  - button "标记保留 frame-003.jpg" [ref=f1e140] [cursor=pointer]
                  - button "标记淘汰 frame-003.jpg" [pressed] [ref=f1e143] [cursor=pointer]
            - article [ref=f1e147]:
              - button "查看第 4 张 frame-004.jpg" [ref=f1e148] [cursor=pointer]:
                - img "frame-004.jpg" [ref=f1e149]
                - generic [ref=f1e150]: "04"
              - generic [ref=f1e151]:
                - generic [ref=f1e152]: FRAME 04
                - generic [ref=f1e153]:
                  - button "标记保留 frame-004.jpg" [ref=f1e154] [cursor=pointer]
                  - button "标记淘汰 frame-004.jpg" [ref=f1e157] [cursor=pointer]
          - generic [ref=f1e161]:
            - article [ref=f1e162]:
              - button "查看第 5 张 frame-005.jpg" [ref=f1e163] [cursor=pointer]:
                - img "frame-005.jpg" [ref=f1e164]
                - generic [ref=f1e165]: "05"
              - generic [ref=f1e166]:
                - generic [ref=f1e167]: FRAME 05
                - generic [ref=f1e168]:
                  - button "标记保留 frame-005.jpg" [ref=f1e169] [cursor=pointer]
                  - button "标记淘汰 frame-005.jpg" [ref=f1e172] [cursor=pointer]
            - article [ref=f1e176]:
              - button "查看第 6 张 frame-006.jpg" [ref=f1e177] [cursor=pointer]:
                - img "frame-006.jpg" [ref=f1e178]
                - generic [ref=f1e179]: "06"
              - generic [ref=f1e180]:
                - generic [ref=f1e181]: FRAME 06
                - generic [ref=f1e182]:
                  - button "标记保留 frame-006.jpg" [ref=f1e183] [cursor=pointer]
                  - button "标记淘汰 frame-006.jpg" [ref=f1e186] [cursor=pointer]
            - article [ref=f1e190]:
              - button "查看第 7 张 frame-007.jpg" [ref=f1e191] [cursor=pointer]:
                - img "frame-007.jpg" [ref=f1e192]
                - generic [ref=f1e193]: "07"
              - generic [ref=f1e194]:
                - generic [ref=f1e195]: FRAME 07
                - generic [ref=f1e196]:
                  - button "标记保留 frame-007.jpg" [ref=f1e197] [cursor=pointer]
                  - button "标记淘汰 frame-007.jpg" [ref=f1e200] [cursor=pointer]
            - article [ref=f1e204]:
              - button "查看第 8 张 frame-008.jpg" [ref=f1e205] [cursor=pointer]:
                - img "frame-008.jpg" [ref=f1e206]
                - generic [ref=f1e207]: "08"
              - generic [ref=f1e208]:
                - generic [ref=f1e209]: FRAME 08
                - generic [ref=f1e210]:
                  - button "标记保留 frame-008.jpg" [ref=f1e211] [cursor=pointer]
                  - button "标记淘汰 frame-008.jpg" [ref=f1e214] [cursor=pointer]
          - generic [ref=f1e218]:
            - article [ref=f1e219]:
              - button "查看第 9 张 frame-009.jpg" [ref=f1e220] [cursor=pointer]:
                - img "frame-009.jpg" [ref=f1e221]
                - generic [ref=f1e222]: "09"
              - generic [ref=f1e223]:
                - generic [ref=f1e224]: FRAME 09
                - generic [ref=f1e225]:
                  - button "标记保留 frame-009.jpg" [ref=f1e226] [cursor=pointer]
                  - button "标记淘汰 frame-009.jpg" [ref=f1e229] [cursor=pointer]
            - article [ref=f1e233]:
              - button "查看第 10 张 frame-010.jpg" [ref=f1e234] [cursor=pointer]:
                - img "frame-010.jpg" [ref=f1e235]
                - generic [ref=f1e236]: "10"
              - generic [ref=f1e237]:
                - generic [ref=f1e238]: FRAME 10
                - generic [ref=f1e239]:
                  - button "标记保留 frame-010.jpg" [ref=f1e240] [cursor=pointer]
                  - button "标记淘汰 frame-010.jpg" [ref=f1e243] [cursor=pointer]
            - article [ref=f1e247]:
              - button "查看第 11 张 frame-011.jpg" [ref=f1e248] [cursor=pointer]:
                - img "frame-011.jpg" [ref=f1e249]
                - generic [ref=f1e250]: "11"
              - generic [ref=f1e251]:
                - generic [ref=f1e252]: FRAME 11
                - generic [ref=f1e253]:
                  - button "标记保留 frame-011.jpg" [ref=f1e254] [cursor=pointer]
                  - button "标记淘汰 frame-011.jpg" [ref=f1e257] [cursor=pointer]
            - article [ref=f1e261]:
              - button "查看第 12 张 frame-012.jpg" [ref=f1e262] [cursor=pointer]:
                - img "frame-012.jpg" [ref=f1e263]
                - generic [ref=f1e264]: "12"
              - generic [ref=f1e265]:
                - generic [ref=f1e266]: FRAME 12
                - generic [ref=f1e267]:
                  - button "标记保留 frame-012.jpg" [ref=f1e268] [cursor=pointer]
                  - button "标记淘汰 frame-012.jpg" [ref=f1e271] [cursor=pointer]
          - generic [ref=f1e275]:
            - article [ref=f1e276]:
              - button "查看第 13 张 frame-013.jpg" [ref=f1e277] [cursor=pointer]:
                - img "frame-013.jpg" [ref=f1e278]
                - generic [ref=f1e279]: "13"
              - generic [ref=f1e280]:
                - generic [ref=f1e281]: FRAME 13
                - generic [ref=f1e282]:
                  - button "标记保留 frame-013.jpg" [ref=f1e283] [cursor=pointer]
                  - button "标记淘汰 frame-013.jpg" [ref=f1e286] [cursor=pointer]
            - article [ref=f1e290]:
              - button "查看第 14 张 frame-014.jpg" [ref=f1e291] [cursor=pointer]:
                - img "frame-014.jpg" [ref=f1e292]
                - generic [ref=f1e293]: "14"
              - generic [ref=f1e294]:
                - generic [ref=f1e295]: FRAME 14
                - generic [ref=f1e296]:
                  - button "标记保留 frame-014.jpg" [ref=f1e297] [cursor=pointer]
                  - button "标记淘汰 frame-014.jpg" [ref=f1e300] [cursor=pointer]
            - article [ref=f1e304]:
              - button "查看第 15 张 frame-015.jpg" [ref=f1e305] [cursor=pointer]:
                - img "frame-015.jpg" [ref=f1e306]
                - generic [ref=f1e307]: "15"
              - generic [ref=f1e308]:
                - generic [ref=f1e309]: FRAME 15
                - generic [ref=f1e310]:
                  - button "标记保留 frame-015.jpg" [ref=f1e311] [cursor=pointer]
                  - button "标记淘汰 frame-015.jpg" [ref=f1e314] [cursor=pointer]
            - article [ref=f1e318]:
              - button "查看第 16 张 frame-016.jpg" [ref=f1e319] [cursor=pointer]:
                - img "frame-016.jpg" [ref=f1e320]
                - generic [ref=f1e321]: "16"
              - generic [ref=f1e322]:
                - generic [ref=f1e323]: FRAME 16
                - generic [ref=f1e324]:
                  - button "标记保留 frame-016.jpg" [ref=f1e325] [cursor=pointer]
                  - button "标记淘汰 frame-016.jpg" [ref=f1e328] [cursor=pointer]
          - generic [ref=f1e332]:
            - article [ref=f1e333]:
              - button "查看第 17 张 frame-017.jpg" [ref=f1e334] [cursor=pointer]:
                - img "frame-017.jpg" [ref=f1e335]
                - generic [ref=f1e336]: "17"
              - generic [ref=f1e337]:
                - generic [ref=f1e338]: FRAME 17
                - generic [ref=f1e339]:
                  - button "标记保留 frame-017.jpg" [ref=f1e340] [cursor=pointer]
                  - button "标记淘汰 frame-017.jpg" [ref=f1e343] [cursor=pointer]
            - article [ref=f1e347]:
              - button "查看第 18 张 frame-018.jpg" [ref=f1e348] [cursor=pointer]:
                - img "frame-018.jpg" [ref=f1e349]
                - generic [ref=f1e350]: "18"
              - generic [ref=f1e351]:
                - generic [ref=f1e352]: FRAME 18
                - generic [ref=f1e353]:
                  - button "标记保留 frame-018.jpg" [ref=f1e354] [cursor=pointer]
                  - button "标记淘汰 frame-018.jpg" [ref=f1e357] [cursor=pointer]
            - article [ref=f1e361]:
              - button "查看第 19 张 frame-019.jpg" [ref=f1e362] [cursor=pointer]:
                - img "frame-019.jpg" [ref=f1e363]
                - generic [ref=f1e364]: "19"
              - generic [ref=f1e365]:
                - generic [ref=f1e366]: FRAME 19
                - generic [ref=f1e367]:
                  - button "标记保留 frame-019.jpg" [ref=f1e368] [cursor=pointer]
                  - button "标记淘汰 frame-019.jpg" [ref=f1e371] [cursor=pointer]
            - article [ref=f1e375]:
              - button "查看第 20 张 frame-020.jpg" [ref=f1e376] [cursor=pointer]:
                - img "frame-020.jpg" [ref=f1e377]
                - generic [ref=f1e378]: "20"
              - generic [ref=f1e379]:
                - generic [ref=f1e380]: FRAME 20
                - generic [ref=f1e381]:
                  - button "标记保留 frame-020.jpg" [ref=f1e382] [cursor=pointer]
                  - button "标记淘汰 frame-020.jpg" [ref=f1e385] [cursor=pointer]
          - generic [ref=f1e389]:
            - article [ref=f1e390]:
              - button "查看第 21 张 frame-021.jpg" [ref=f1e391] [cursor=pointer]:
                - img "frame-021.jpg" [ref=f1e392]
                - generic [ref=f1e393]: "21"
              - generic [ref=f1e394]:
                - generic [ref=f1e395]: FRAME 21
                - generic [ref=f1e396]:
                  - button "标记保留 frame-021.jpg" [ref=f1e397] [cursor=pointer]
                  - button "标记淘汰 frame-021.jpg" [ref=f1e400] [cursor=pointer]
            - article [ref=f1e404]:
              - button "查看第 22 张 frame-022.jpg" [ref=f1e405] [cursor=pointer]:
                - img "frame-022.jpg" [ref=f1e406]
                - generic [ref=f1e407]: "22"
              - generic [ref=f1e408]:
                - generic [ref=f1e409]: FRAME 22
                - generic [ref=f1e410]:
                  - button "标记保留 frame-022.jpg" [ref=f1e411] [cursor=pointer]
                  - button "标记淘汰 frame-022.jpg" [ref=f1e414] [cursor=pointer]
            - article [ref=f1e418]:
              - button "查看第 23 张 frame-023.jpg" [ref=f1e419] [cursor=pointer]:
                - img "frame-023.jpg" [ref=f1e420]
                - generic [ref=f1e421]: "23"
              - generic [ref=f1e422]:
                - generic [ref=f1e423]: FRAME 23
                - generic [ref=f1e424]:
                  - button "标记保留 frame-023.jpg" [ref=f1e425] [cursor=pointer]
                  - button "标记淘汰 frame-023.jpg" [ref=f1e428] [cursor=pointer]
            - article [ref=f1e432]:
              - button "查看第 24 张 frame-024.jpg" [ref=f1e433] [cursor=pointer]:
                - img "frame-024.jpg" [ref=f1e434]
                - generic [ref=f1e435]: "24"
              - generic [ref=f1e436]:
                - generic [ref=f1e437]: FRAME 24
                - generic [ref=f1e438]:
                  - button "标记保留 frame-024.jpg" [ref=f1e439] [cursor=pointer]
                  - button "标记淘汰 frame-024.jpg" [ref=f1e442] [cursor=pointer]
      - generic [ref=f1e446]:
        - generic [ref=f1e447]:
          - strong [ref=f1e448]: Selected 00
          - text: 作品初选序列
        - generic [ref=f1e449]: 保留照片后将在这里形成作品序列
        - button "编辑序列 →" [ref=f1e451] [cursor=pointer]
  - complementary [ref=f1e452]:
    - text: THE EDITOR / WORKING NOTES
    - heading "每一次选择， 都是你的表达。" [level=2] [ref=f1e453]
    - paragraph [ref=f1e454]: 保留你的判断，慢慢找到画面之间的关系。
    - generic "检查器面板" [ref=f1e455]:
      - button "照片检查" [ref=f1e456] [cursor=pointer]
      - button "AI 建议" [ref=f1e457] [cursor=pointer]
      - button "审美记忆" [pressed] [ref=f1e458] [cursor=pointer]
    - generic [ref=f1e459]:
      - heading "选择有来处，偏好有证据。" [level=3] [ref=f1e460]
      - paragraph [ref=f1e461]: 假设为项目内关键词规则推断，可确认、纠正或移除。排除证据只停止参与学习；原始人工记录、任务历史与备份仍保留。
      - generic [ref=f1e462]:
        - generic [ref=f1e463]:
          - text: YOUR CHOICES
          - strong [ref=f1e464]: "1"
          - text: 人工选择
        - generic [ref=f1e465]:
          - text: COMPARISONS
          - strong [ref=f1e466]: "0"
          - text: 比较记录
        - generic [ref=f1e467]:
          - text: HYPOTHESES
          - strong [ref=f1e468]: "1"
          - text: 偏好假设
      - paragraph [ref=f1e469]: 调整记忆会暂停当前 Agent，请基于当前记忆重新委托。选片、作品序列与撤销历史保持原样。
      - generic [ref=f1e470]:
        - generic [ref=f1e471]:
          - checkbox "允许离线 Agent 引用其他本地项目记忆" [disabled] [ref=f1e472]
          - text: 允许离线 Agent 引用其他本地项目记忆
        - paragraph [ref=f1e473]: 明确开启后，离线 Agent 可引用其他本地项目已确认的相关假设；共享/迁移中项目与云模型不参与。当前桌面按单一本地作者处理。
      - generic [ref=f1e474]:
        - generic [ref=f1e475]:
          - text: 搜索偏好关键词
          - textbox "搜索偏好关键词" [ref=f1e476]:
            - /placeholder: 例如：留白、逆光
        - button "搜索" [disabled] [ref=f1e477]
      - generic [ref=f1e478]:
        - checkbox "显示已移除假设" [ref=f1e479]
        - text: 显示已移除假设
      - generic [ref=f1e481]: PREFERENCE HYPOTHESES / 偏好假设
      - article [ref=f1e482]:
        - strong [ref=f1e483]: 本项目多次人工判断的说明中出现「留白」
        - paragraph [ref=f1e484]: 未确认 · 初步置信 · 证据 2 条 · 反例 1 条
        - group [ref=f1e485]:
          - generic "查看证据" [ref=f1e486]
        - group [ref=f1e487]:
          - generic "查看反例" [ref=f1e488]
        - generic [ref=f1e489]:
          - button "确认此假设" [disabled] [ref=f1e490]
          - button "移除此假设" [disabled] [ref=f1e491]
        - generic [ref=f1e492]:
          - button "纠正此假设" [disabled] [ref=f1e493]
          - button "停用检索" [disabled] [ref=f1e494]
          - button "删除假设" [disabled] [ref=f1e495]
      - generic [ref=f1e497]: EVIDENCE / 人工证据
      - generic [ref=f1e498]:
        - button "清空视觉观察缓存" [disabled] [ref=f1e499]
        - generic [ref=f1e500]: 不含证据与假设；删除本机缓存的观察结果及相关检索缓存。
      - list [ref=f1e501]:
        - listitem [ref=f1e502]:
          - time [ref=f1e503]: 2026/10/17 03:56:57
          - paragraph [ref=f1e504]: "#28 · 备注 · frame-003.jpg"
          - paragraph [ref=f1e505]: 留白太多
          - 'button "排除证据 #28" [disabled] [ref=f1e506]': 排除学习
          - 'button "删除证据 #28" [disabled] [ref=f1e507]': 删除
        - listitem [ref=f1e508]:
          - time [ref=f1e509]: 2026/10/17 03:56:57
          - paragraph [ref=f1e510]: "#27 · 人工淘汰 · frame-003.jpg"
          - paragraph [ref=f1e511]: 该操作没有留下文字理由。
          - 'button "排除证据 #27" [disabled] [ref=f1e512]': 排除学习
          - 'button "删除证据 #27" [disabled] [ref=f1e513]': 删除
        - listitem [ref=f1e514]:
          - time [ref=f1e515]: 2026/10/17 03:56:57
          - paragraph [ref=f1e516]: "#26 · 备注 · frame-002.jpg"
          - paragraph [ref=f1e517]: 留白
          - 'button "排除证据 #26" [disabled] [ref=f1e518]': 排除学习
          - 'button "删除证据 #26" [disabled] [ref=f1e519]': 删除
        - listitem [ref=f1e520]:
          - time [ref=f1e521]: 2026/10/17 03:56:57
          - paragraph [ref=f1e522]: "#25 · 备注 · frame-001.jpg"
          - paragraph [ref=f1e523]: 留白
          - 'button "排除证据 #25" [disabled] [ref=f1e524]': 排除学习
          - 'button "删除证据 #25" [disabled] [ref=f1e525]': 删除
      - paragraph [ref=f1e526]: 证据仅记录你的明确操作；Agent 的草案与建议不会出现在这里。撤销或重做不重复计数。
```

# Test source

```ts
  1   | // M4 本地记忆：浏览器界面驱动真实 Rust/SQLite，禁止外发。
  2   | import { test, expect } from '@playwright/test';
  3   | import { mkdir } from 'node:fs/promises';
  4   | import { resolve } from 'node:path';
  5   | import type { MemoryProjection } from '../src/domain/memory';
  6   | 
  7   | async function rpc<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  8   |   const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  9   |   if (!response.ok) throw new Error(response.error.message);
  10  |   return response.value;
  11  | }
  12  | type Project = { id: string; revision: number; sequence: string[]; photos: { id: string; status: string; note: string }[] };
  13  | 
  14  | test('M4 记忆控制：确认、反例、排除/恢复、移除/恢复、搜索及重启', async ({ page }) => {
  15  |   const screenshots = resolve('../docs/screenshots/m4-memory-controls');
  16  |   await mkdir(screenshots, { recursive: true });
  17  |   const errors: string[] = []; const outside: string[] = [];
  18  |   page.on('pageerror', e => errors.push(e.message));
  19  |   await page.route('**/*', route => {
  20  |     if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  21  |     outside.push(route.request().url()); return route.abort();
  22  |   });
  23  |   await page.addInitScript(() => Object.assign(globalThis, {
  24  |     isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
  25  |     __TAURI_INTERNALS__: {
  26  |       invoke: async (command: string, args: unknown) => {
  27  |         if (command.startsWith('plugin:event|')) return 0;
  28  |         const reply = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  29  |         if (!reply.ok) throw reply.error; return reply.value;
  30  |       },
  31  |       convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
  32  |     },
  33  |   }));
  34  |   let project = await rpc<Project>('create_project', { title: 'M4 · 选择有证据', brief: '人工证据可排除、假设可纠正与恢复。' });
  35  |   const hashes = await rpc<string[][]>('test_fixture_checksums');
  36  |   const selectionToken = await rpc<string>('choose_import', { projectId: project.id });
  37  |   const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken });
  38  |   await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100000 }).toBe('completed');
  39  |   project = await rpc<Project>('get_project', { projectId: project.id });
  40  |   for (const index of [0, 1]) project = await rpc<Project>('update_photo_note', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[index].id, text: '留白' });
  41  |   project = await rpc<Project>('update_photo_decision', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[2].id, status: 'reject' });
  42  |   project = await rpc<Project>('update_photo_note', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[2].id, text: '留白太多' });
  43  |   const formal = await rpc<Project>('get_project', { projectId: project.id });
  44  |   const initial = await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id });
  45  |   const h = initial.hypotheses.find(h => h.statement.includes('留白'))!;
  46  |   const support = h.evidenceIds[0];
  47  |   await page.goto('/#/project/' + project.id);
  48  |   await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  49  |   const card = page.locator('.memory-hypothesis').filter({ hasText: h.statement });
  50  |   await expect(card).toContainText('证据 2 条 · 反例 1 条');
  51  |   await card.getByText('查看反例', { exact: true }).click();
  52  |   await expect(card).toContainText('留白太多');
  53  |   await card.getByRole('button', { name: '确认此假设' }).click();
  54  |   await expect(card).toHaveAttribute('data-status', 'confirmed');
  55  |   await page.screenshot({ path: resolve(screenshots, 'confirmed-counterexample.png') });
  56  |   await page.getByRole('button', { name: '排除证据 #' + support, exact: true }).click();
  57  |   await expect(card).toContainText('证据 1 条');
  58  |   await page.getByRole('button', { name: '恢复证据 #' + support, exact: true }).click();
  59  |   await expect(card).toContainText('证据 2 条');
  60  |   await card.getByRole('button', { name: '移除此假设' }).click();
  61  |   await expect(card).toHaveCount(0);
  62  |   await page.getByRole('checkbox', { name: '显示已移除假设' }).check();
  63  |   await expect(card).toHaveAttribute('data-status', 'retired');
  64  |   await card.getByRole('button', { name: '恢复此假设' }).click();
  65  |   await expect(card).toHaveAttribute('data-status', 'unconfirmed');
  66  |   await page.getByRole('textbox', { name: '搜索偏好关键词' }).fill('逆光');
  67  |   await page.getByRole('button', { name: '搜索', exact: true }).click();
  68  |   await expect(page.getByText('没有匹配的假设')).toBeVisible();
  69  |   await page.getByRole('textbox', { name: '搜索偏好关键词' }).fill('留白');
  70  |   await page.getByRole('button', { name: '搜索', exact: true }).click();
  71  |   await expect(card).toBeVisible();
  72  |   await page.screenshot({ path: resolve(screenshots, 'restored-search.png') });
  73  |   const memory = await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id });
  74  |   expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  75  |   await rpc('test_restart');
  76  |   expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  77  |   expect(await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id })).toEqual(memory);
  78  |   expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  79  |   await page.reload();
  80  |   await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  81  |   await expect(card).toHaveAttribute('data-status', 'unconfirmed');
  82  |   // 作者记忆授权通过真实 Rust 命令，来源卡保持只读并跨进程重启保留。
  83  |   let source = await rpc<Project>('create_project', { title: '作者记忆来源', brief: '仅临时测试素材' });
  84  |   const sourceToken = await rpc<string>('choose_import', { projectId: source.id });
  85  |   const sourceJob = await rpc<string>('import_photos', { projectId: source.id, selectionToken: sourceToken });
  86  |   await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId: sourceJob })).status, { timeout: 100000 }).toBe('completed');
  87  |   source = await rpc<Project>('get_project', { projectId: source.id });
  88  |   for (const index of [0, 1]) source = await rpc<Project>('update_photo_note', { projectId: source.id, expectedRevision: source.revision, photoId: source.photos[index].id, text: '留白' });
  89  |   const sourceHypothesis = (await rpc<MemoryProjection>('get_memory_projection', { projectId: source.id })).hypotheses.find(h => h.statement.includes('留白'))!;
  90  |   const confirmed = await rpc<{ ok: boolean }>('execute_memory_command', { request: { requestId: crypto.randomUUID(), projectId: source.id, command: { type: 'ConfirmHypothesis', hypothesisId: sourceHypothesis.id } } });
  91  |   expect(confirmed.ok).toBe(true);
  92  |   const toggle = page.getByRole('checkbox', { name: '允许离线 Agent 引用其他本地项目记忆' });
  93  |   await expect(toggle).not.toBeChecked();
  94  |   page.once('dialog', dialog => dialog.dismiss());
  95  |   await toggle.click();
  96  |   await expect(toggle).not.toBeChecked();
  97  |   page.once('dialog', dialog => dialog.accept());
> 98  |   await toggle.check();
      |                ^ Error: locator.check: Clicking the checkbox did not change its state
  99  |   const author = page.locator('[data-author-memory="' + sourceHypothesis.id + '"]');
  100 |   await expect(author).toContainText(source.id);
  101 |   await expect(author.getByRole('link')).toHaveAttribute('href', '#/project/' + source.id);
  102 |   await expect(author.getByRole('button')).toHaveCount(0);
  103 |   await mkdir(resolve('../docs/evidence/v2-m3-author-memory'), { recursive: true });
  104 |   await page.screenshot({ path: resolve('../docs/evidence/v2-m3-author-memory/author-memory-ui.png') });
  105 |   await rpc('test_restart');
  106 |   await page.reload();
  107 |   await page.getByRole('button', { name: '审美记忆', exact: true }).last().click();
  108 |   await expect(toggle).toBeChecked();
  109 |   await expect(author).toBeVisible();
  110 |   await toggle.uncheck();
  111 |   await expect(author).toHaveCount(0);
  112 |   expect((await rpc<MemoryProjection>('get_memory_projection', { projectId: project.id })).crossProjectEnabled).toBe(false);
  113 |   expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(formal);
  114 |   expect(await rpc<Project>('get_project', { projectId: source.id })).toEqual(source);
  115 |   expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  116 |   expect(errors).toEqual([]); expect(outside).toEqual([]);
  117 | });
  118 | 
```