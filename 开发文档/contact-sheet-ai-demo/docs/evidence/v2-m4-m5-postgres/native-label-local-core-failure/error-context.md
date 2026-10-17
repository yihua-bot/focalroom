# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: local-core.spec.ts >> v1.0 M1 真实 Store 离线旅程：200 张、批量选 25、Undo/Redo、版本恢复、私密导出与重启
- Location: e2e/local-core.spec.ts:13:1

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.project-card').filter({ hasText: '光线的间隙 · M1 验收' }).getByText('LOCAL PROJECT', { exact: true })
Expected: visible
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" locator('.project-card').filter({ hasText: '光线的间隙 · M1 验收' }).getByText('LOCAL PROJECT', { exact: true }) with timeout 5000ms
  - waiting for locator('.project-card').filter({ hasText: '光线的间隙 · M1 验收' }).getByText('LOCAL PROJECT', { exact: true })

```

```yaml
- complementary:
  - img "FocalRoom 标志"
  - text: FocalRoom PHOTO STUDIO
  - button "收起导航":
    - img
  - text: WORKSPACE
  - navigation "主导航":
    - button "选片工作台" [disabled]:
      - img
      - text: 选片工作台
    - button "作品序列" [disabled]:
      - img
      - text: 作品序列
    - button "审美记忆" [disabled]:
      - img
      - text: 审美记忆
    - button "项目列表":
      - img
      - text: 项目列表
  - text: CURRENT PROJECT
  - button "快捷键帮助":
    - img
    - text: 快捷键帮助
  - text: 本机保存 · 原图只读 图像上传需授权
- main:
  - text: YOUR PROJECTS / FOCALROOM
  - heading "你的创作，从这里继续。" [level=1]
  - paragraph: 每个项目，都是一次观看与取舍。
  - button "新建项目":
    - img
    - text: 新建项目
  - paragraph: 本机项目缓存 · 打开项目查看共享状态 · 原图只读
  - button "归档项目"
  - article:
    - button "frame-000.jpg PROJECT CACHE 光线的间隙 · M1 验收 200 张图像 · 25 张入选 · 本机项目缓存 最近修改 2026/10/17 23:37:36":
      - img "frame-000.jpg"
      - text: PROJECT CACHE
      - heading "光线的间隙 · M1 验收" [level=2]:
        - text: 光线的间隙 · M1 验收
        - img
      - paragraph: 200 张图像 · 25 张入选 · 本机项目缓存
      - paragraph: 最近修改 2026/10/17 23:37:36
    - button "归档项目"
  - article:
    - button "从一张照片开始。 PROJECT CACHE M5 · 私密项目名称不进入诊断 0 张图像 · 0 张入选 · 本机项目缓存 最近修改 2026/10/17 23:37:25":
      - text: 从一张照片开始。 PROJECT CACHE
      - heading "M5 · 私密项目名称不进入诊断" [level=2]:
        - text: M5 · 私密项目名称不进入诊断
        - img
      - paragraph: 0 张图像 · 0 张入选 · 本机项目缓存
      - paragraph: 最近修改 2026/10/17 23:37:25
    - button "归档项目"
  - article:
    - button "frame-000.jpg PROJECT CACHE 成长的摄影书 200 张图像 · 11 张入选 · 本机项目缓存 最近修改 2026/10/17 23:37:23":
      - img "frame-000.jpg"
      - text: PROJECT CACHE
      - heading "成长的摄影书" [level=2]:
        - text: 成长的摄影书
        - img
      - paragraph: 200 张图像 · 11 张入选 · 本机项目缓存
      - paragraph: 最近修改 2026/10/17 23:37:23
    - button "归档项目"
  - article:
    - button "frame-000.jpg PROJECT CACHE M2 · 草案与人工决定 200 张图像 · 25 张入选 · 本机项目缓存 最近修改 2026/10/17 23:37:12":
      - img "frame-000.jpg"
      - text: PROJECT CACHE
      - heading "M2 · 草案与人工决定" [level=2]:
        - text: M2 · 草案与人工决定
        - img
      - paragraph: 200 张图像 · 25 张入选 · 本机项目缓存
      - paragraph: 最近修改 2026/10/17 23:37:12
    - button "归档项目"
```

# Test source

```ts
  62  |   await expect(page.getByRole('button', { name: '保存我的判断' })).toBeDisabled();
  63  |   for (const index of [3, 5, 7, 10, 13]) {
  64  |     await page.getByTestId(`photo-${index}`).getByRole('button', { name: /^查看第/ }).click();
  65  |     await page.keyboard.press('k');
  66  |     await expect(page.getByTestId(`photo-${index}`)).toHaveClass(/keep/);
  67  |   }
  68  |   await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  69  |   await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = 0; });
  70  |   await page.screenshot({ path: resolve(screenshots, 'workspace.png') });
  71  |   await page.getByRole('button', { name: '两图对比' }).click();
  72  |   await expect(page.getByRole('dialog').getByRole('img', { name: 'frame-000.jpg' })).toHaveJSProperty('naturalWidth', 480);
  73  |   await page.screenshot({ path: resolve(screenshots, 'compare.png') });
  74  |   await page.getByRole('button', { name: '偏好右张' }).click();
  75  |   await expect(page.getByRole('dialog')).toHaveCount(0);
  76  |   await expect(page.getByRole('button', { name: /^已保留\s*6$/ })).toBeVisible();
  77  |   await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  78  |   for (let index = 1; index < 25; index++) await page.keyboard.press('Shift+ArrowRight');
  79  |   await expect(page.getByRole('group', { name: '批量选片' })).toContainText('已选 25 张');
  80  |   await page.getByRole('button', { name: '保留选区' }).click();
  81  |   await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  82  |   const kept = await rpc<StoredProject>('get_project', { projectId });
  83  |   await page.keyboard.press('k');
  84  |   expect(await rpc<StoredProject>('get_project', { projectId })).toEqual(kept);
  85  |   await page.getByRole('combobox', { name: '批量人工评分' }).selectOption('4');
  86  |   await expect(page.locator('.tile-rating').first()).toContainText('★★★★');
  87  |   await page.getByRole('button', { name: '作品序列', exact: true }).click();
  88  |   await expect(page.getByTestId('sequence-card')).toHaveCount(25);
  89  |   await page.getByRole('button', { name: '下移 frame-000.jpg' }).click();
  90  |   await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  91  |   await page.mouse.move(1100, 30);
  92  |   await page.screenshot({ path: resolve(screenshots, 'sequence.png') });
  93  |   await page.getByRole('button', { name: /^作品版本/ }).click();
  94  |   await page.getByRole('textbox', { name: '版本名称' }).fill('25 张 · 初选');
  95  |   await page.getByRole('button', { name: '保存当前版本' }).click();
  96  |   await expect(page.getByRole('button', { name: /25 张 · 初选/ })).toBeVisible();
  97  |   await page.getByRole('button', { name: /25 张 · 初选/ }).click();
  98  |   await page.screenshot({ path: resolve(screenshots, 'versions.png') });
  99  |   await page.keyboard.press('Escape');
  100 |   await page.getByRole('button', { name: '上移 frame-000.jpg' }).click();
  101 |   await expect(page.getByTestId('sequence-card').first()).toContainText('frame-000.jpg');
  102 |   await page.getByRole('button', { name: '撤销', exact: true }).click();
  103 |   await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  104 |   await page.getByRole('button', { name: '重做', exact: true }).click();
  105 |   await expect(page.getByTestId('sequence-card').first()).toContainText('frame-000.jpg');
  106 |   await page.getByRole('button', { name: /^作品版本/ }).click();
  107 |   await page.getByRole('button', { name: /25 张 · 初选/ }).click();
  108 |   await expect(page.getByRole('region', { name: '版本恢复预览' })).toContainText('位置变化 2 张');
  109 |   await page.getByRole('button', { name: '确认恢复此版本' }).click();
  110 |   await expect(page.getByRole('dialog')).toHaveCount(0);
  111 |   await expect(page.getByTestId('sequence-card').nth(1)).toContainText('frame-000.jpg');
  112 |   const before = await rpc<StoredProject>('get_project', { projectId });
  113 |   await rpc('test_restart');
  114 |   await page.reload();
  115 |   const after = await rpc<StoredProject>('get_project', { projectId });
  116 |   expect(after).toEqual(before);
  117 |   await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  118 |   await expect(page.getByRole('textbox', { name: '为什么保留 / 淘汰这张照片？' })).toHaveValue('保留这张，让光线成为序列中的一次停顿。');
  119 |   expect(new Set(after.events.map(event => event.kind))).toEqual(new Set(['project_create', 'import', 'decision', 'note', 'compare', 'reorder', 'rating', 'version_save', 'undo', 'redo', 'version_restore']));
  120 |   expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(checksums);
  121 |   for (const width of [1920, 1280]) {
  122 |     await page.setViewportSize({ width, height: width === 1920 ? 1080 : 800 });
  123 |     await expect.poll(async () => {
  124 |       const rows = page.locator('.virtual-row');
  125 |       const first = await rows.nth(0).boundingBox();
  126 |       const second = await rows.nth(1).boundingBox();
  127 |       return first && second ? second.y - first.y - first.height : 1000;
  128 |     }).toBeLessThan(40);
  129 |     await page.screenshot({ path: resolve(screenshots, `workspace-${width}.png`) });
  130 |     expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  131 |   }
  132 |   await page.getByRole('button', { name: '导出 ↗' }).click();
  133 |   for (const name of ['项目 JSON', '选片 CSV', '接触印相']) {
  134 |     await page.getByRole('button', { name, exact: true }).click();
  135 |     await expect(page.locator('.export-receipt')).toContainText('已导出：');
  136 |     const text = await page.locator('.export-receipt p').innerText();
  137 |     const bytes = await readFile(text.replace('已导出：', ''));
  138 |     const hash = createHash('sha256').update(bytes).digest('hex');
  139 |     if (name === '项目 JSON') {
  140 |       const exported = JSON.parse(bytes.toString('utf8'));
  141 |       expect(exported.includesLocalPaths).toBe(false);
  142 |       expect(exported.photos[0]).not.toHaveProperty('originalPath');
  143 |       expect(bytes.toString('utf8')).not.toContain(before.photos[0].originalPath);
  144 |     }
  145 |     if (name === '选片 CSV') expect(bytes.toString('utf8').split('\r\n')[0]).not.toContain('original_path');
  146 |     await expect(page.locator('.export-receipt')).toContainText(hash);
  147 |   }
  148 |   await page.keyboard.press('Escape');
  149 |   await rpc('test_move_source', { projectId });
  150 |   await page.reload();
  151 |   await expect(page.getByText('原图失联，缓存预览仍可查看。')).toBeVisible();
  152 |   await page.getByRole('button', { name: '重新定位原图' }).click();
  153 |   await expect(page.getByText('原图失联，缓存预览仍可查看。')).toHaveCount(0);
  154 |   expect((await rpc<StoredProject>('get_project', { projectId })).photos[0].id).toBe(before.photos[0].id);
  155 |   await page.getByRole('button', { name: '项目列表', exact: true }).click();
  156 |   const ownCard = page.locator('.project-card').filter({ hasText: '光线的间隙 · M1 验收' });
  157 |   await ownCard.getByRole('button', { name: '归档项目', exact: true }).click();
  158 |   await page.getByRole('button', { name: '确认归档', exact: true }).click();
  159 |   await expect(ownCard).toHaveCount(0);
  160 |   await page.locator('.project-list-tools').getByRole('button', { name: '归档项目', exact: true }).click();
  161 |   await ownCard.getByRole('button', { name: '恢复项目', exact: true }).click();
> 162 |   await expect(ownCard.getByText('LOCAL PROJECT', { exact: true })).toBeVisible();
      |                                                                     ^ Error: expect(locator).toBeVisible() failed
  163 |   expect(errors).toEqual([]);
  164 |   expect(externalRequests).toEqual([]);
  165 | });
  166 | 
```