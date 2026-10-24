# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: beta-journey.spec.ts >> Beta 旅程：选片、委托、采纳、纠正假设、导出与重启一致
- Location: e2e/beta-journey.spec.ts:17:1

# Error details

```
TimeoutError: locator.click: Timeout 15000ms exceeded.
Call log:
  - waiting for getByRole('main').getByRole('button', { name: 'Focal AI', exact: true })
    - locator resolved to <button type="button" class="button primary">…</button>
  - attempting click action
    2 × waiting for element to be visible, enabled and stable
      - element is visible, enabled and stable
      - scrolling into view if needed
      - done scrolling
      - <header class="ai-drawer-header">…</header> from <div role="dialog" tabindex="-1" id="radix-_r_1_" data-state="open" class="ai-drawer" aria-labelledby="radix-_r_2_">…</div> subtree intercepts pointer events
    - retrying click action
    - waiting 20ms
    2 × waiting for element to be visible, enabled and stable
      - element is visible, enabled and stable
      - scrolling into view if needed
      - done scrolling
      - <header class="ai-drawer-header">…</header> from <div role="dialog" tabindex="-1" id="radix-_r_1_" data-state="open" class="ai-drawer" aria-labelledby="radix-_r_2_">…</div> subtree intercepts pointer events
    - retrying click action
      - waiting 100ms
    29 × waiting for element to be visible, enabled and stable
       - element is visible, enabled and stable
       - scrolling into view if needed
       - done scrolling
       - <header class="ai-drawer-header">…</header> from <div role="dialog" tabindex="-1" id="radix-_r_1_" data-state="open" class="ai-drawer" aria-labelledby="radix-_r_2_">…</div> subtree intercepts pointer events
     - retrying click action
       - waiting 500ms

```

# Page snapshot

```yaml
- generic [active] [ref=e1]:
  - generic [ref=e3]:
    - complementary [ref=e4]:
      - generic [ref=e5]:
        - img "FocalRoom 标志" [ref=e6]
        - generic [ref=e7]:
          - generic [ref=e8]: FocalRoom
          - generic [ref=e9]: PHOTO STUDIO
      - button "收起导航" [ref=e10] [cursor=pointer]
      - navigation "项目列表" [ref=e14]:
        - button "项目列表" [ref=e15] [cursor=pointer]
      - generic [ref=e20]: 当前项目
      - generic [ref=e21]:
        - strong [ref=e22]: 成长的摄影书
        - generic [ref=e23]: 200 张图像 · 8 张入选本地项目
      - navigation "成长的摄影书" [ref=e24]:
        - button "选片工作台" [ref=e25] [cursor=pointer]
        - button "作品序列" [ref=e29] [cursor=pointer]
        - button "叙事画布" [ref=e34] [cursor=pointer]
        - button "出版物" [ref=e41] [cursor=pointer]
      - button "账户菜单" [ref=e48] [cursor=pointer]:
        - generic [aria-hidden] [ref=e49]: "Y"
        - generic [ref=e50]:
          - strong [ref=e51]: Yihua
          - generic [ref=e52]: 基础版
    - main [ref=e55]:
      - generic [ref=e56]:
        - link "← 项目列表" [ref=e57] [cursor=pointer]:
          - /url: "#/projects"
        - generic [ref=e58]:
          - heading "成长的摄影书" [level=1] [ref=e60]
          - generic [ref=e61]:
            - button "导入照片" [ref=e62] [cursor=pointer]
            - button "Focal AI" [ref=e64] [cursor=pointer]
        - generic [ref=e67]:
          - generic [ref=e68]: “
          - paragraph [ref=e69]: 克制、日常、有时间感的家庭影像
          - button "编辑创作意图 ↗" [ref=e70] [cursor=pointer]
      - group "编辑历史" [ref=e71]:
        - button "撤销" [ref=e72] [cursor=pointer]
        - button "重做" [disabled] [ref=e73]
        - button "作品版本" [ref=e74] [cursor=pointer]
        - button "调色版本" [ref=e75] [cursor=pointer]
        - button "操作历史 · 212" [ref=e76] [cursor=pointer]
        - generic [ref=e77]: 11 次可撤销编辑
      - generic [ref=e78]:
        - generic "照片筛选" [ref=e79]:
          - button "全部 200" [pressed] [ref=e80] [cursor=pointer]:
            - text: 全部
            - generic [ref=e81]: "200"
          - button "已保留 8" [ref=e82] [cursor=pointer]:
            - text: 已保留
            - generic [ref=e83]: "8"
          - button "待定 192" [ref=e84] [cursor=pointer]:
            - text: 待定
            - generic [ref=e85]: "192"
          - button "已淘汰 0" [ref=e86] [cursor=pointer]:
            - text: 已淘汰
            - generic [ref=e87]: "0"
        - generic [ref=e88]:
          - button "切换网格密度" [ref=e89] [cursor=pointer]
          - button "两图对比" [ref=e92] [cursor=pointer]
          - button "导出 ↗" [ref=e93] [cursor=pointer]
          - button "收起序列带" [pressed] [ref=e94] [cursor=pointer]
          - button "收起侧栏" [ref=e97] [cursor=pointer]
        - generic [ref=e101]:
          - searchbox "搜索照片文件名或备注" [ref=e106]
          - generic [ref=e107]:
            - text: 分组
            - combobox "网格分组" [ref=e108]:
              - option "不分组" [selected]
              - option "按选片状态"
              - option "按人工评分"
              - option "按文件名前缀"
          - generic [ref=e109]: ⌘ / Ctrl 点选 · Shift 连选
      - generic [ref=e110]:
        - generic [ref=e111]:
          - generic [ref=e112]:
            - generic [ref=e113]: CONTACT SHEET
            - generic [ref=e114]: 200 张
          - generic "照片网格" [ref=e116]:
            - generic [ref=e117]:
              - article [ref=e118]:
                - button "查看第 1 张 frame-000.jpg" [ref=e119] [cursor=pointer]:
                  - img "frame-000.jpg" [ref=e120]
                  - generic [ref=e121]: "01"
                  - generic [ref=e122]: ◇ 已保留
                - generic [ref=e123]:
                  - generic [ref=e124]: FRAME 01
                  - generic [ref=e125]:
                    - button "标记保留 frame-000.jpg" [pressed] [ref=e126] [cursor=pointer]
                    - button "标记淘汰 frame-000.jpg" [ref=e129] [cursor=pointer]
              - article [ref=e133]:
                - button "查看第 2 张 frame-001.jpg" [ref=e134] [cursor=pointer]:
                  - img "frame-001.jpg" [ref=e135]
                  - generic [ref=e136]: "02"
                  - generic [ref=e137]: ◇ 已保留
                - generic [ref=e138]:
                  - generic [ref=e139]: FRAME 02
                  - generic [ref=e140]:
                    - button "标记保留 frame-001.jpg" [pressed] [ref=e141] [cursor=pointer]
                    - button "标记淘汰 frame-001.jpg" [ref=e144] [cursor=pointer]
              - article [ref=e148]:
                - button "查看第 3 张 frame-002.jpg" [ref=e149] [cursor=pointer]:
                  - img "frame-002.jpg" [ref=e150]
                  - generic [ref=e151]: "03"
                  - generic [ref=e152]: ◇ 已保留
                - generic [ref=e153]:
                  - generic [ref=e154]: FRAME 03
                  - generic [ref=e155]:
                    - button "标记保留 frame-002.jpg" [pressed] [ref=e156] [cursor=pointer]
                    - button "标记淘汰 frame-002.jpg" [ref=e159] [cursor=pointer]
              - article [ref=e163]:
                - button "查看第 4 张 frame-003.jpg" [ref=e164] [cursor=pointer]:
                  - img "frame-003.jpg" [ref=e165]
                  - generic [ref=e166]: "04"
                  - generic [ref=e167]: ◇ 已保留
                - generic [ref=e168]:
                  - generic [ref=e169]: FRAME 04
                  - generic [ref=e170]:
                    - button "标记保留 frame-003.jpg" [pressed] [ref=e171] [cursor=pointer]
                    - button "标记淘汰 frame-003.jpg" [ref=e174] [cursor=pointer]
            - generic [ref=e178]:
              - article [ref=e179]:
                - button "查看第 5 张 frame-004.jpg" [ref=e180] [cursor=pointer]:
                  - img "frame-004.jpg" [ref=e181]
                  - generic [ref=e182]: "05"
                  - generic [ref=e183]: ◇ 已保留
                - generic [ref=e184]:
                  - generic [ref=e185]: FRAME 05
                  - generic [ref=e186]:
                    - button "标记保留 frame-004.jpg" [pressed] [ref=e187] [cursor=pointer]
                    - button "标记淘汰 frame-004.jpg" [ref=e190] [cursor=pointer]
              - article [ref=e194]:
                - button "查看第 6 张 frame-005.jpg" [ref=e195] [cursor=pointer]:
                  - img "frame-005.jpg" [ref=e196]
                  - generic [ref=e197]: "06"
                  - generic [ref=e198]: ◇ 已保留
                - generic [ref=e199]:
                  - generic [ref=e200]: FRAME 06
                  - generic [ref=e201]:
                    - button "标记保留 frame-005.jpg" [pressed] [ref=e202] [cursor=pointer]
                    - button "标记淘汰 frame-005.jpg" [ref=e205] [cursor=pointer]
              - article [ref=e209]:
                - button "查看第 7 张 frame-006.jpg" [ref=e210] [cursor=pointer]:
                  - img "frame-006.jpg" [ref=e211]
                  - generic [ref=e212]: "07"
                  - generic [ref=e213]: ◇ 已保留
                - generic [ref=e214]:
                  - generic [ref=e215]: FRAME 07
                  - generic [ref=e216]:
                    - button "标记保留 frame-006.jpg" [pressed] [ref=e217] [cursor=pointer]
                    - button "标记淘汰 frame-006.jpg" [ref=e220] [cursor=pointer]
              - article [ref=e224]:
                - button "查看第 8 张 frame-007.jpg" [ref=e225] [cursor=pointer]:
                  - img "frame-007.jpg" [ref=e226]
                  - generic [ref=e227]: "08"
                  - generic [ref=e228]: ◇ 已保留
                - generic [ref=e229]:
                  - generic [ref=e230]: FRAME 08
                  - generic [ref=e231]:
                    - button "标记保留 frame-007.jpg" [pressed] [ref=e232] [cursor=pointer]
                    - button "标记淘汰 frame-007.jpg" [ref=e235] [cursor=pointer]
            - generic [ref=e239]:
              - article [ref=e240]:
                - button "查看第 9 张 frame-008.jpg" [ref=e241] [cursor=pointer]:
                  - img "frame-008.jpg" [ref=e242]
                  - generic [ref=e243]: "09"
                - generic [ref=e244]:
                  - generic [ref=e245]: FRAME 09
                  - generic [ref=e246]:
                    - button "标记保留 frame-008.jpg" [ref=e247] [cursor=pointer]
                    - button "标记淘汰 frame-008.jpg" [ref=e250] [cursor=pointer]
              - article [ref=e254]:
                - button "查看第 10 张 frame-009.jpg" [ref=e255] [cursor=pointer]:
                  - img "frame-009.jpg" [ref=e256]
                  - generic [ref=e257]: "10"
                - generic [ref=e258]:
                  - generic [ref=e259]: FRAME 10
                  - generic [ref=e260]:
                    - button "标记保留 frame-009.jpg" [ref=e261] [cursor=pointer]
                    - button "标记淘汰 frame-009.jpg" [ref=e264] [cursor=pointer]
              - article [ref=e268]:
                - button "查看第 11 张 frame-010.jpg" [ref=e269] [cursor=pointer]:
                  - img "frame-010.jpg" [ref=e270]
                  - generic [ref=e271]: "11"
                - generic [ref=e272]:
                  - generic [ref=e273]: FRAME 11
                  - generic [ref=e274]:
                    - button "标记保留 frame-010.jpg" [ref=e275] [cursor=pointer]
                    - button "标记淘汰 frame-010.jpg" [ref=e278] [cursor=pointer]
              - article [ref=e282]:
                - button "查看第 12 张 frame-011.jpg" [ref=e283] [cursor=pointer]:
                  - img "frame-011.jpg" [ref=e284]
                  - generic [ref=e285]: "12"
                - generic [ref=e286]:
                  - generic [ref=e287]: FRAME 12
                  - generic [ref=e288]:
                    - button "标记保留 frame-011.jpg" [ref=e289] [cursor=pointer]
                    - button "标记淘汰 frame-011.jpg" [ref=e292] [cursor=pointer]
            - generic [ref=e296]:
              - article [ref=e297]:
                - button "查看第 13 张 frame-012.jpg" [ref=e298] [cursor=pointer]:
                  - img "frame-012.jpg" [ref=e299]
                  - generic [ref=e300]: "13"
                - generic [ref=e301]:
                  - generic [ref=e302]: FRAME 13
                  - generic [ref=e303]:
                    - button "标记保留 frame-012.jpg" [ref=e304] [cursor=pointer]
                    - button "标记淘汰 frame-012.jpg" [ref=e307] [cursor=pointer]
              - article [ref=e311]:
                - button "查看第 14 张 frame-013.jpg" [ref=e312] [cursor=pointer]:
                  - img "frame-013.jpg" [ref=e313]
                  - generic [ref=e314]: "14"
                - generic [ref=e315]:
                  - generic [ref=e316]: FRAME 14
                  - generic [ref=e317]:
                    - button "标记保留 frame-013.jpg" [ref=e318] [cursor=pointer]
                    - button "标记淘汰 frame-013.jpg" [ref=e321] [cursor=pointer]
              - article [ref=e325]:
                - button "查看第 15 张 frame-014.jpg" [ref=e326] [cursor=pointer]:
                  - img "frame-014.jpg" [ref=e327]
                  - generic [ref=e328]: "15"
                - generic [ref=e329]:
                  - generic [ref=e330]: FRAME 15
                  - generic [ref=e331]:
                    - button "标记保留 frame-014.jpg" [ref=e332] [cursor=pointer]
                    - button "标记淘汰 frame-014.jpg" [ref=e335] [cursor=pointer]
              - article [ref=e339]:
                - button "查看第 16 张 frame-015.jpg" [ref=e340] [cursor=pointer]:
                  - img "frame-015.jpg" [ref=e341]
                  - generic [ref=e342]: "16"
                - generic [ref=e343]:
                  - generic [ref=e344]: FRAME 16
                  - generic [ref=e345]:
                    - button "标记保留 frame-015.jpg" [ref=e346] [cursor=pointer]
                    - button "标记淘汰 frame-015.jpg" [ref=e349] [cursor=pointer]
            - generic [ref=e353]:
              - article [ref=e354]:
                - button "查看第 17 张 frame-016.jpg" [ref=e355] [cursor=pointer]:
                  - img "frame-016.jpg" [ref=e356]
                  - generic [ref=e357]: "17"
                - generic [ref=e358]:
                  - generic [ref=e359]: FRAME 17
                  - generic [ref=e360]:
                    - button "标记保留 frame-016.jpg" [ref=e361] [cursor=pointer]
                    - button "标记淘汰 frame-016.jpg" [ref=e364] [cursor=pointer]
              - article [ref=e368]:
                - button "查看第 18 张 frame-017.jpg" [ref=e369] [cursor=pointer]:
                  - img "frame-017.jpg" [ref=e370]
                  - generic [ref=e371]: "18"
                - generic [ref=e372]:
                  - generic [ref=e373]: FRAME 18
                  - generic [ref=e374]:
                    - button "标记保留 frame-017.jpg" [ref=e375] [cursor=pointer]
                    - button "标记淘汰 frame-017.jpg" [ref=e378] [cursor=pointer]
              - article [ref=e382]:
                - button "查看第 19 张 frame-018.jpg" [ref=e383] [cursor=pointer]:
                  - img "frame-018.jpg" [ref=e384]
                  - generic [ref=e385]: "19"
                - generic [ref=e386]:
                  - generic [ref=e387]: FRAME 19
                  - generic [ref=e388]:
                    - button "标记保留 frame-018.jpg" [ref=e389] [cursor=pointer]
                    - button "标记淘汰 frame-018.jpg" [ref=e392] [cursor=pointer]
              - article [ref=e396]:
                - button "查看第 20 张 frame-019.jpg" [ref=e397] [cursor=pointer]:
                  - img "frame-019.jpg" [ref=e398]
                  - generic [ref=e399]: "20"
                - generic [ref=e400]:
                  - generic [ref=e401]: FRAME 20
                  - generic [ref=e402]:
                    - button "标记保留 frame-019.jpg" [ref=e403] [cursor=pointer]
                    - button "标记淘汰 frame-019.jpg" [ref=e406] [cursor=pointer]
            - generic [ref=e410]:
              - article [ref=e411]:
                - button "查看第 21 张 frame-020.jpg" [ref=e412] [cursor=pointer]:
                  - img "frame-020.jpg" [ref=e413]
                  - generic [ref=e414]: "21"
                - generic [ref=e415]:
                  - generic [ref=e416]: FRAME 21
                  - generic [ref=e417]:
                    - button "标记保留 frame-020.jpg" [ref=e418] [cursor=pointer]
                    - button "标记淘汰 frame-020.jpg" [ref=e421] [cursor=pointer]
              - article [ref=e425]:
                - button "查看第 22 张 frame-021.jpg" [ref=e426] [cursor=pointer]:
                  - img "frame-021.jpg" [ref=e427]
                  - generic [ref=e428]: "22"
                - generic [ref=e429]:
                  - generic [ref=e430]: FRAME 22
                  - generic [ref=e431]:
                    - button "标记保留 frame-021.jpg" [ref=e432] [cursor=pointer]
                    - button "标记淘汰 frame-021.jpg" [ref=e435] [cursor=pointer]
              - article [ref=e439]:
                - button "查看第 23 张 frame-022.jpg" [ref=e440] [cursor=pointer]:
                  - img "frame-022.jpg" [ref=e441]
                  - generic [ref=e442]: "23"
                - generic [ref=e443]:
                  - generic [ref=e444]: FRAME 23
                  - generic [ref=e445]:
                    - button "标记保留 frame-022.jpg" [ref=e446] [cursor=pointer]
                    - button "标记淘汰 frame-022.jpg" [ref=e449] [cursor=pointer]
              - article [ref=e453]:
                - button "查看第 24 张 frame-023.jpg" [ref=e454] [cursor=pointer]:
                  - img "frame-023.jpg" [ref=e455]
                  - generic [ref=e456]: "24"
                - generic [ref=e457]:
                  - generic [ref=e458]: FRAME 24
                  - generic [ref=e459]:
                    - button "标记保留 frame-023.jpg" [ref=e460] [cursor=pointer]
                    - button "标记淘汰 frame-023.jpg" [ref=e463] [cursor=pointer]
        - generic [ref=e467]:
          - generic [ref=e468]:
            - strong [ref=e469]: Selected 08
            - text: 作品初选序列
          - generic [ref=e470]:
            - button "查看入选 frame-000.jpg" [ref=e471] [cursor=pointer]:
              - img "frame-000.jpg" [ref=e472]
            - button "查看入选 frame-001.jpg" [ref=e473] [cursor=pointer]:
              - img "frame-001.jpg" [ref=e474]
            - button "查看入选 frame-002.jpg" [ref=e475] [cursor=pointer]:
              - img "frame-002.jpg" [ref=e476]
            - button "查看入选 frame-003.jpg" [ref=e477] [cursor=pointer]:
              - img "frame-003.jpg" [ref=e478]
            - button "查看入选 frame-004.jpg" [ref=e479] [cursor=pointer]:
              - img "frame-004.jpg" [ref=e480]
            - button "查看入选 frame-005.jpg" [ref=e481] [cursor=pointer]:
              - img "frame-005.jpg" [ref=e482]
            - button "查看入选 frame-006.jpg" [ref=e483] [cursor=pointer]:
              - img "frame-006.jpg" [ref=e484]
            - button "查看入选 frame-007.jpg" [ref=e485] [cursor=pointer]:
              - img "frame-007.jpg" [ref=e486]
          - button "编辑序列 →" [ref=e487] [cursor=pointer]
    - complementary "照片详情" [ref=e488]:
      - heading "照片详情" [level=2] [ref=e489]
      - region "照片详情" [ref=e490]:
        - button "查看完整大图" [ref=e491] [cursor=pointer]:
          - img "当前选中：frame-000.jpg" [ref=e492]
        - generic [ref=e498]: FRAME 01
        - heading "frame-000.jpg" [level=3] [ref=e499]
        - generic [ref=e500]: 已保留
        - paragraph [ref=e501]: 480 × 320
        - group "人工星级" [ref=e502]:
          - generic [ref=e503]: 你的评分
          - button "人工评分 1 星" [ref=e504] [cursor=pointer]: ★
          - button "人工评分 2 星" [ref=e505] [cursor=pointer]: ★
          - button "人工评分 3 星" [ref=e506] [cursor=pointer]: ★
          - button "人工评分 4 星" [ref=e507] [cursor=pointer]: ★
          - button "人工评分 5 星" [ref=e508] [cursor=pointer]: ★
          - button "清除人工评分" [disabled] [ref=e509]: 清除
        - group "册页标记" [ref=e510]:
          - generic [ref=e511]: 册页（印刷）
          - button "跨页大图" [ref=e512] [cursor=pointer]
          - button "整页图" [ref=e513] [cursor=pointer]
          - generic [ref=e514]: 默认每页两张
        - generic [ref=e515]:
          - button "◇ 保留作品" [ref=e516] [cursor=pointer]
          - button "× 淘汰作品" [ref=e517] [cursor=pointer]
          - button "恢复待定" [ref=e518] [cursor=pointer]
        - generic [ref=e519]: 照片备注
        - textbox "照片备注" [ref=e520]:
          - /placeholder: 构图、光线、拍摄背景…
          - text: 窗边的光线有层次，日常但克制
        - button "保存备注" [disabled] [ref=e522]
  - dialog [ref=e523]:
    - banner [ref=e524]:
      - generic [ref=e528]:
        - heading "Focal AI" [level=2] [ref=e529]
        - paragraph [ref=e530]: 成长的摄影书
      - button "审美记忆" [expanded] [ref=e531] [cursor=pointer]
      - button "关闭 Focal AI" [ref=e535] [cursor=pointer]
    - region "审美记忆" [ref=e539]:
      - generic [ref=e540]:
        - button "返回对话" [ref=e541] [cursor=pointer]
        - heading "审美记忆" [level=3] [ref=e544]
      - generic [ref=e545]:
        - generic [ref=e546]:
          - generic [ref=e547]:
            - strong [ref=e548]: "8"
            - text: 人工选择
          - generic [ref=e549]:
            - strong [ref=e550]: "0"
            - text: 比较记录
          - generic [ref=e551]:
            - strong [ref=e552]: "5"
            - text: 偏好假设
        - paragraph [ref=e553]: 修改记忆后，需重新启动进行中的任务。
        - generic [ref=e554]:
          - generic [ref=e555]:
            - checkbox "参考其他本地项目的偏好" [ref=e556]
            - text: 参考其他本地项目的偏好
          - paragraph [ref=e557]: 仅用于本机任务，不发送给云端模型。
        - generic [ref=e558]:
          - generic [ref=e559]:
            - text: 搜索偏好关键词
            - textbox "搜索偏好关键词" [ref=e560]:
              - /placeholder: 例如：留白、逆光
          - button "搜索" [ref=e561] [cursor=pointer]
        - generic [ref=e562]:
          - checkbox "显示已移除假设" [ref=e563]
          - text: 显示已移除假设
        - generic [ref=e565]: 偏好假设
        - article [ref=e566]:
          - strong [ref=e567]: 本项目多次人工判断的说明中出现「常但」
          - paragraph [ref=e568]: 已纠正 · 初步置信 · 证据 3 条 · 已停用检索
          - note [ref=e569]: 你的纠正：光线只是巧合，不是我的稳定偏好
          - group [ref=e570]:
            - generic "查看证据" [ref=e571]
          - button "移除此假设" [ref=e573] [cursor=pointer]
          - button "删除假设" [ref=e575] [cursor=pointer]
        - article [ref=e576]:
          - strong [ref=e577]: 本项目多次人工判断的说明中出现「层次」
          - paragraph [ref=e578]: 未确认 · 初步置信 · 证据 3 条
          - group [ref=e579]:
            - generic "查看证据" [ref=e580]
          - generic [ref=e581]:
            - button "确认此假设" [ref=e582] [cursor=pointer]
            - button "移除此假设" [ref=e583] [cursor=pointer]
          - generic [ref=e584]:
            - button "纠正此假设" [ref=e585] [cursor=pointer]
            - button "停用检索" [ref=e586] [cursor=pointer]
            - button "删除假设" [ref=e587] [cursor=pointer]
        - article [ref=e588]:
          - strong [ref=e589]: 本项目多次人工判断的说明中出现「克制」
          - paragraph [ref=e590]: 未确认 · 初步置信 · 证据 3 条
          - group [ref=e591]:
            - generic "查看证据" [ref=e592]
          - generic [ref=e593]:
            - button "确认此假设" [ref=e594] [cursor=pointer]
            - button "移除此假设" [ref=e595] [cursor=pointer]
          - generic [ref=e596]:
            - button "纠正此假设" [ref=e597] [cursor=pointer]
            - button "停用检索" [ref=e598] [cursor=pointer]
            - button "删除假设" [ref=e599] [cursor=pointer]
        - article [ref=e600]:
          - strong [ref=e601]: 本项目多次人工判断的说明中出现「光线」
          - paragraph [ref=e602]: 未确认 · 初步置信 · 证据 3 条
          - group [ref=e603]:
            - generic "查看证据" [ref=e604]
          - generic [ref=e605]:
            - button "确认此假设" [ref=e606] [cursor=pointer]
            - button "移除此假设" [ref=e607] [cursor=pointer]
          - generic [ref=e608]:
            - button "纠正此假设" [ref=e609] [cursor=pointer]
            - button "停用检索" [ref=e610] [cursor=pointer]
            - button "删除假设" [ref=e611] [cursor=pointer]
        - article [ref=e612]:
          - strong [ref=e613]: 本项目多次人工判断的说明中出现「但克」
          - paragraph [ref=e614]: 未确认 · 初步置信 · 证据 3 条
          - group [ref=e615]:
            - generic "查看证据" [ref=e616]
          - generic [ref=e617]:
            - button "确认此假设" [ref=e618] [cursor=pointer]
            - button "移除此假设" [ref=e619] [cursor=pointer]
          - generic [ref=e620]:
            - button "纠正此假设" [ref=e621] [cursor=pointer]
            - button "停用检索" [ref=e622] [cursor=pointer]
            - button "删除假设" [ref=e623] [cursor=pointer]
        - generic [ref=e625]: 人工证据
        - button "清空视觉观察缓存" [ref=e627] [cursor=pointer]
        - list [ref=e628]:
          - listitem [ref=e629]:
            - time [ref=e630]: 2026/10/24 20:15:41
            - paragraph [ref=e631]: "#14 · 备注 · frame-002.jpg"
            - paragraph [ref=e632]: 窗边的光线有层次，日常但克制
            - 'button "排除证据 #14" [ref=e633] [cursor=pointer]': 排除学习
            - 'button "删除证据 #14" [ref=e634] [cursor=pointer]': 删除
          - listitem [ref=e635]:
            - time [ref=e636]: 2026/10/24 20:15:41
            - paragraph [ref=e637]: "#13 · 备注 · frame-001.jpg"
            - paragraph [ref=e638]: 窗边的光线有层次，日常但克制
            - 'button "排除证据 #13" [ref=e639] [cursor=pointer]': 排除学习
            - 'button "删除证据 #13" [ref=e640] [cursor=pointer]': 删除
          - listitem [ref=e641]:
            - time [ref=e642]: 2026/10/24 20:15:41
            - paragraph [ref=e643]: "#12 · 备注 · frame-000.jpg"
            - paragraph [ref=e644]: 窗边的光线有层次，日常但克制
            - 'button "排除证据 #12" [ref=e645] [cursor=pointer]': 排除学习
            - 'button "删除证据 #12" [ref=e646] [cursor=pointer]': 删除
          - listitem [ref=e647]:
            - time [ref=e648]: 2026/10/24 20:15:41
            - paragraph [ref=e649]: "#11 · 人工保留 · frame-007.jpg"
            - paragraph [ref=e650]: 该操作没有留下文字理由。
            - 'button "排除证据 #11" [ref=e651] [cursor=pointer]': 排除学习
            - 'button "删除证据 #11" [ref=e652] [cursor=pointer]': 删除
          - listitem [ref=e653]:
            - time [ref=e654]: 2026/10/24 20:15:41
            - paragraph [ref=e655]: "#10 · 人工保留 · frame-006.jpg"
            - paragraph [ref=e656]: 该操作没有留下文字理由。
            - 'button "排除证据 #10" [ref=e657] [cursor=pointer]': 排除学习
            - 'button "删除证据 #10" [ref=e658] [cursor=pointer]': 删除
          - listitem [ref=e659]:
            - time [ref=e660]: 2026/10/24 20:15:41
            - paragraph [ref=e661]: "#9 · 人工保留 · frame-005.jpg"
            - paragraph [ref=e662]: 该操作没有留下文字理由。
            - 'button "排除证据 #9" [ref=e663] [cursor=pointer]': 排除学习
            - 'button "删除证据 #9" [ref=e664] [cursor=pointer]': 删除
          - listitem [ref=e665]:
            - time [ref=e666]: 2026/10/24 20:15:41
            - paragraph [ref=e667]: "#8 · 人工保留 · frame-004.jpg"
            - paragraph [ref=e668]: 该操作没有留下文字理由。
            - 'button "排除证据 #8" [ref=e669] [cursor=pointer]': 排除学习
            - 'button "删除证据 #8" [ref=e670] [cursor=pointer]': 删除
          - listitem [ref=e671]:
            - time [ref=e672]: 2026/10/24 20:15:41
            - paragraph [ref=e673]: "#7 · 人工保留 · frame-003.jpg"
            - paragraph [ref=e674]: 该操作没有留下文字理由。
            - 'button "排除证据 #7" [ref=e675] [cursor=pointer]': 排除学习
            - 'button "删除证据 #7" [ref=e676] [cursor=pointer]': 删除
          - listitem [ref=e677]:
            - time [ref=e678]: 2026/10/24 20:15:41
            - paragraph [ref=e679]: "#6 · 人工保留 · frame-002.jpg"
            - paragraph [ref=e680]: 该操作没有留下文字理由。
            - 'button "排除证据 #6" [ref=e681] [cursor=pointer]': 排除学习
            - 'button "删除证据 #6" [ref=e682] [cursor=pointer]': 删除
          - listitem [ref=e683]:
            - time [ref=e684]: 2026/10/24 20:15:41
            - paragraph [ref=e685]: "#5 · 人工保留 · frame-001.jpg"
            - paragraph [ref=e686]: 该操作没有留下文字理由。
            - 'button "排除证据 #5" [ref=e687] [cursor=pointer]': 排除学习
            - 'button "删除证据 #5" [ref=e688] [cursor=pointer]': 删除
          - listitem [ref=e689]:
            - time [ref=e690]: 2026/10/24 20:15:41
            - paragraph [ref=e691]: "#4 · 人工保留 · frame-000.jpg"
            - paragraph [ref=e692]: 该操作没有留下文字理由。
            - 'button "排除证据 #4" [ref=e693] [cursor=pointer]': 排除学习
            - 'button "删除证据 #4" [ref=e694] [cursor=pointer]': 删除
```

# Test source

```ts
  1   | import { evidenceDirectory } from './evidence-directory';
  2   | import { installAccountFixture, installDesktopBridge } from './account-fixture';
  3   | // M5 Beta 完整旅程：意图 → 人工选片与备注 → Agent 委托 → 局部采纳 →
  4   | // 记忆假设纠正 → 导出 → 重启一致性。真实 Rust 后端（stdio harness）。
  5   | import { test, expect } from '@playwright/test';
  6   | import { mkdir, readFile } from 'node:fs/promises';
  7   | import { resolve, join } from 'node:path';
  8   | 
  9   | const rpcBase = process.env.FOCALROOM_TEST_RPC_BASE ?? 'http://localhost:1422';
  10  | const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  11  |   const response = await fetch(rpcBase + '/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  12  |   if (!response.ok) throw new Error(response.error.message);
  13  |   return response.value;
  14  | };
  15  | type Project = { id: string; revision: number; sequence: string[]; photos: { id: string; status: string; note: string; rating: number }[] };
  16  | 
  17  | test('Beta 旅程：选片、委托、采纳、纠正假设、导出与重启一致', async ({ page }) => {
  18  |   const screenshots = evidenceDirectory('beta-journey');
  19  |   await mkdir(screenshots, { recursive: true });
  20  |   await page.route('**/*', route => {
  21  |     if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  22  |     return route.abort('internetdisconnected');
  23  |   });
  24  |   await installAccountFixture(page);
  25  |  await installDesktopBridge(page,rpcBase);
  26  |   // 1. 建项目 + 写创作意图。
  27  |   const project = await rpc<Project>('create_project', { title: '成长的摄影书', brief: '克制、日常、有时间感的家庭影像' });
  28  |   const token = await rpc<string>('choose_import', { projectId: project.id });
  29  |   const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken: token });
  30  |   await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 120000 }).toBe('completed');
  31  |   let current = await rpc<Project>('get_project', { projectId: project.id });
  32  |   expect(current.photos).toHaveLength(200);
  33  |   // 2. 人工保留 8 张，其中 3 张带共同关键词备注（记忆素材）。
  34  |   const picked = current.photos.slice(0, 8).map(p => p.id);
  35  |   for (const photoId of picked) {
  36  |     current = await rpc<Project>('execute_gui_command', { request: { commandId: `beta-${photoId}`, projectId: project.id, expectedProjectRevision: current.revision, causationId: null, idempotencyKey: `beta-${photoId}`, command: { type: 'SetPhotoDecision', photoIds: [photoId], status: 'keep' } } });
  37  |   }
  38  |   for (const photoId of picked.slice(0, 3)) {
  39  |     current = await rpc<Project>('execute_gui_command', { request: { commandId: `beta-note-${photoId}`, projectId: project.id, expectedProjectRevision: current.revision, causationId: null, idempotencyKey: `beta-note-${photoId}`, command: { type: 'SetPhotoNote', photoId, note: '窗边的光线有层次，日常但克制' } } });
  40  |   }
  41  |   // 3. 委托 Agent（本地 Mock，零图像）。
  42  |   await page.goto('/#/project/' + project.id);
  43  |   await expect(page.getByRole('heading', { name: '成长的摄影书' })).toBeVisible();
  44  |   await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
  45  |   await page.getByRole('textbox', { name: '消息' }).fill('整理成长的摄影书，突出窗边的光线');
  46  |   await page.getByRole('button', { name: '发送', exact: true }).click();
  47  |   await expect(page.locator('.chat-task-status')).toHaveText('待审核', { timeout: 30000 });
  48  |   await expect(page.locator('.chat-draft-card')).toHaveCount(2);
  49  |   await page.screenshot({ path: resolve(screenshots, 'journey-drafts.png') });
  50  |   // 4. 记忆面板出现人工证据与假设，纠正一条假设（AG-11 旅程级）。
  51  |   await page.getByRole('button', { name: '审美记忆', exact: true }).click();
  52  |   await expect(page.getByText('人工证据')).toBeVisible();
  53  |   const memory = await rpc<{ evidence: unknown[]; hypotheses: { id: number; statement: string }[] }>('get_memory_projection', { projectId: project.id });
  54  |   expect(memory.evidence.length).toBeGreaterThanOrEqual(11);
  55  |   const target = memory.hypotheses.find(h => h.statement.includes('层次') || memory.hypotheses[0]);
  56  |   expect(target).toBeTruthy();
  57  |   await page.getByRole('button', { name: '纠正此假设' }).first().click();
  58  |   await page.getByRole('textbox', { name: '纠正说明' }).fill('光线只是巧合，不是我的稳定偏好');
  59  |   await page.getByRole('button', { name: '提交纠正' }).click();
  60  |   await expect(page.getByText('已纠正').first()).toBeVisible({ timeout: 10000 });
  61  |   const corrected = await rpc<{ hypotheses: { id: number; status: string; disabled: boolean }[] }>('get_memory_projection', { projectId: project.id });
  62  |   const after = corrected.hypotheses.find(h => h.id === target!.id);
  63  |   expect(after?.status).toBe('corrected');
  64  |   expect(after?.disabled).toBe(true);
  65  |   await page.screenshot({ path: resolve(screenshots, 'journey-memory.png') });
  66  |   // 5. 局部采纳一份草案的部分照片。
> 67  |   await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
      |                                                                                       ^ TimeoutError: locator.click: Timeout 15000ms exceeded.
  68  |   await page.getByRole('button', { name: '查看方案' }).first().click();
  69  |   await page.getByRole('button', { name: '清除选择' }).click();
  70  |   const checkboxes = page.locator('.draft-items input[type="checkbox"]');
  71  |   const total = await checkboxes.count();
  72  |   expect(total).toBeGreaterThan(3);
  73  |   for (let i = 0; i < 3; i++) await checkboxes.nth(i).check();
  74  |   await page.getByRole('button', { name: '采纳所选 3 张' }).click();
  75  |   await page.waitForTimeout(2000);
  76  |   await page.screenshot({ path: resolve(screenshots, 'journey-applied.png') });
  77  |   // 6. 导出 CSV（默认私密）。
  78  |   const exported = await rpc<{ path: string }>('export_project', { projectId: project.id, format: 'csv', includePaths: false });
  79  |   expect(await readFile(exported.path, 'utf8')).not.toContain('original_path');
  80  |   // 6.1 关闭选片抽屉后从工作台导出。
  81  |   await page.getByRole('button', { name: '关闭 Focal AI' }).click();
  82  |   // 印刷参考走当前 GUI、IPC 和 Rust 导出器。
  83  |   await page.getByRole('button', { name: '导出 ↗' }).click();
  84  |   await page.getByRole('button', { name: '印刷参考', exact: true }).click();
  85  |   await expect(page.locator('.export-receipt')).toContainText('已导出：');
  86  |   const printPath = (await page.locator('.export-receipt p').innerText()).replace('已导出：', '');
  87  |   expect(printPath).toMatch(/\.html$/);
  88  |   const printHtml = await readFile(printPath, 'utf8');
  89  |   expect(printHtml).toContain('class="sheet"');
  90  |   // 实际排版成 PDF：HTML 节点数正确仍可能因页脚溢出而多印空白页。
  91  |   const printPage = await page.context().newPage();
  92  |   try {
  93  |     await printPage.setContent(printHtml);
  94  |     const pdf = await printPage.pdf({ preferCSSPageSize: true, printBackground: true });
  95  |     const pages = pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length ?? 0;
  96  |     expect(pages).toBe(1 + (printHtml.match(/class="sheet"/g)?.length ?? 0));
  97  |   } finally { await printPage.close(); }
  98  |   // 侧车不再是前端选项，后端兼容导出仍验证文件数量与路径脱敏。
  99  |   const sidecar = await rpc<{ path: string }>('export_project', { projectId: project.id, format: 'sidecar', includePaths: false });
  100 |   const sidecarPath = sidecar.path;
  101 |   const manifest = JSON.parse(await readFile(join(sidecarPath, 'manifest.json'), 'utf8'));
  102 |   expect(manifest.photos).toBe(200);
  103 |   expect(manifest.files).toHaveLength(200);
  104 |   expect(JSON.parse(await readFile(join(sidecarPath, manifest.files[0].file), 'utf8'))).not.toHaveProperty('originalPath');
  105 |   // 7. 重启一致性：重开数据库后证据/纠正/正式状态保持。
  106 |   const beforeRestart = await rpc<Project>('get_project', { projectId: project.id });
  107 |   await rpc('test_restart');
  108 |   expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(beforeRestart);
  109 |   const memoryAfter = await rpc<{ evidence: unknown[]; hypotheses: { id: number; status: string }[] }>('get_memory_projection', { projectId: project.id });
  110 |   expect(memoryAfter.hypotheses.find(h => h.id === target!.id)?.status).toBe('corrected');
  111 |   // 诊断为 harness 全局计数（同 run 前序 spec 的数据也计入），断言用下界。
  112 |   const diagnostics = await rpc<{ database: { photos: number; evidence: number }; provider: { providerId: string } }>('get_diagnostics');
  113 |   expect(diagnostics.database.photos).toBeGreaterThanOrEqual(200);
  114 |   expect(diagnostics.database.evidence).toBeGreaterThanOrEqual(11);
  115 |   expect(diagnostics.provider.providerId).toBe('mock');
  116 | });
  117 | 
```