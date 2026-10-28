# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: publication-studio.spec.ts >> 真实照片版本回执丢失：不重复切换，返回后仅确认原请求
- Location: e2e/publication-studio.spec.ts:283:1

# Error details

```
Error: expect(locator).toBeDisabled() failed

Locator: getByRole('dialog', { name: '图像版本', exact: true }).getByRole('button', { name: '使用此版本', exact: true })
Expected: disabled
Timeout: 5000ms
Error: element(s) not found

Call log:
  - Expect "toBeDisabled" getByRole('dialog', { name: '图像版本', exact: true }).getByRole('button', { name: '使用此版本', exact: true }) with timeout 5000ms
  - waiting for getByRole('dialog', { name: '图像版本', exact: true }).getByRole('button', { name: '使用此版本', exact: true })

```

```yaml
- complementary:
  - img "FocalRoom 标志"
  - text: FocalRoom PHOTO STUDIO
  - button "收起导航":
    - img
  - navigation "项目列表":
    - button "项目列表":
      - img
      - text: 项目列表
  - text: 当前项目
  - strong: 出版版本 a84660
  - text: 200 张图像 · 3 张入选 本地项目
  - navigation "出版版本 a84660":
    - button "选片工作台":
      - img
      - text: 选片工作台
    - button "作品序列":
      - img
      - text: 作品序列
    - button "叙事画布":
      - img
      - text: 叙事画布
    - button "出版物":
      - img
      - text: 出版物
    - button "共享评片":
      - img
      - text: 共享评片
  - button "账户菜单":
    - img
    - strong: 未登录
    - text: 登录以使用 AI 与协作
    - img
- main:
  - link "← 项目列表":
    - /url: "#/projects"
  - heading "出版版本 a84660" [level=1]
  - button "导入照片":
    - img
    - text: 导入照片
  - button "Focal AI":
    - img
    - text: Focal AI
  - text: “
  - paragraph: 记录版面变化，保留原图和已确认的版本
  - button "编辑创作意图 ↗"
  - group "编辑历史":
    - button "撤销"
    - button "重做" [disabled]
    - button "作品版本"
    - button "调色版本"
    - button "操作历史 · 202"
    - text: 1 次可撤销编辑
  - button "全部 200" [pressed]
  - button "已保留 3"
  - button "待定 197"
  - button "已淘汰 0"
  - button "切换网格密度":
    - img
  - button "两图对比"
  - button "导出 ↗"
  - button "收起序列带" [pressed]:
    - img
  - button "收起侧栏":
    - img
  - img
  - searchbox "搜索照片文件名或备注"
  - text: 分组
  - combobox "网格分组":
    - option "不分组" [selected]
    - option "按选片状态"
    - option "按人工评分"
    - option "按文件名前缀"
  - text: ⌘ / Ctrl 点选 · Shift 连选 选片工作台 200 张
  - article:
    - button "查看第 1 张 frame-000.jpg":
      - img "frame-000.jpg"
      - text: 01 ◇ 已保留
    - text: FRAME 01
    - button "标记保留 frame-000.jpg" [pressed]:
      - img
    - button "标记淘汰 frame-000.jpg":
      - img
  - article:
    - button "查看第 2 张 frame-001.jpg":
      - img "frame-001.jpg"
      - text: 02 ◇ 已保留
    - text: FRAME 02
    - button "标记保留 frame-001.jpg" [pressed]:
      - img
    - button "标记淘汰 frame-001.jpg":
      - img
  - article:
    - button "查看第 3 张 frame-002.jpg":
      - img "frame-002.jpg"
      - text: 03 ◇ 已保留
    - text: FRAME 03
    - button "标记保留 frame-002.jpg" [pressed]:
      - img
    - button "标记淘汰 frame-002.jpg":
      - img
  - article:
    - button "查看第 4 张 frame-003.jpg":
      - img "frame-003.jpg"
      - text: "04"
    - text: FRAME 04
    - button "标记保留 frame-003.jpg":
      - img
    - button "标记淘汰 frame-003.jpg":
      - img
  - article:
    - button "查看第 5 张 frame-004.jpg":
      - img "frame-004.jpg"
      - text: "05"
    - text: FRAME 05
    - button "标记保留 frame-004.jpg":
      - img
    - button "标记淘汰 frame-004.jpg":
      - img
  - article:
    - button "查看第 6 张 frame-005.jpg":
      - img "frame-005.jpg"
      - text: "06"
    - text: FRAME 06
    - button "标记保留 frame-005.jpg":
      - img
    - button "标记淘汰 frame-005.jpg":
      - img
  - article:
    - button "查看第 7 张 frame-006.jpg":
      - img "frame-006.jpg"
      - text: "07"
    - text: FRAME 07
    - button "标记保留 frame-006.jpg":
      - img
    - button "标记淘汰 frame-006.jpg":
      - img
  - article:
    - button "查看第 8 张 frame-007.jpg":
      - img "frame-007.jpg"
      - text: "08"
    - text: FRAME 08
    - button "标记保留 frame-007.jpg":
      - img
    - button "标记淘汰 frame-007.jpg":
      - img
  - article:
    - button "查看第 9 张 frame-008.jpg":
      - img "frame-008.jpg"
      - text: "09"
    - text: FRAME 09
    - button "标记保留 frame-008.jpg":
      - img
    - button "标记淘汰 frame-008.jpg":
      - img
  - article:
    - button "查看第 10 张 frame-009.jpg":
      - img "frame-009.jpg"
      - text: "10"
    - text: FRAME 10
    - button "标记保留 frame-009.jpg":
      - img
    - button "标记淘汰 frame-009.jpg":
      - img
  - article:
    - button "查看第 11 张 frame-010.jpg":
      - img "frame-010.jpg"
      - text: "11"
    - text: FRAME 11
    - button "标记保留 frame-010.jpg":
      - img
    - button "标记淘汰 frame-010.jpg":
      - img
  - article:
    - button "查看第 12 张 frame-011.jpg":
      - img "frame-011.jpg"
      - text: "12"
    - text: FRAME 12
    - button "标记保留 frame-011.jpg":
      - img
    - button "标记淘汰 frame-011.jpg":
      - img
  - article:
    - button "查看第 13 张 frame-012.jpg":
      - img "frame-012.jpg"
      - text: "13"
    - text: FRAME 13
    - button "标记保留 frame-012.jpg":
      - img
    - button "标记淘汰 frame-012.jpg":
      - img
  - article:
    - button "查看第 14 张 frame-013.jpg":
      - img "frame-013.jpg"
      - text: "14"
    - text: FRAME 14
    - button "标记保留 frame-013.jpg":
      - img
    - button "标记淘汰 frame-013.jpg":
      - img
  - article:
    - button "查看第 15 张 frame-014.jpg":
      - img "frame-014.jpg"
      - text: "15"
    - text: FRAME 15
    - button "标记保留 frame-014.jpg":
      - img
    - button "标记淘汰 frame-014.jpg":
      - img
  - article:
    - button "查看第 16 张 frame-015.jpg":
      - img "frame-015.jpg"
      - text: "16"
    - text: FRAME 16
    - button "标记保留 frame-015.jpg":
      - img
    - button "标记淘汰 frame-015.jpg":
      - img
  - article:
    - button "查看第 17 张 frame-016.jpg":
      - img "frame-016.jpg"
      - text: "17"
    - text: FRAME 17
    - button "标记保留 frame-016.jpg":
      - img
    - button "标记淘汰 frame-016.jpg":
      - img
  - article:
    - button "查看第 18 张 frame-017.jpg":
      - img "frame-017.jpg"
      - text: "18"
    - text: FRAME 18
    - button "标记保留 frame-017.jpg":
      - img
    - button "标记淘汰 frame-017.jpg":
      - img
  - article:
    - button "查看第 19 张 frame-018.jpg":
      - img "frame-018.jpg"
      - text: "19"
    - text: FRAME 19
    - button "标记保留 frame-018.jpg":
      - img
    - button "标记淘汰 frame-018.jpg":
      - img
  - article:
    - button "查看第 20 张 frame-019.jpg":
      - img "frame-019.jpg"
      - text: "20"
    - text: FRAME 20
    - button "标记保留 frame-019.jpg":
      - img
    - button "标记淘汰 frame-019.jpg":
      - img
  - article:
    - button "查看第 21 张 frame-020.jpg":
      - img "frame-020.jpg"
      - text: "21"
    - text: FRAME 21
    - button "标记保留 frame-020.jpg":
      - img
    - button "标记淘汰 frame-020.jpg":
      - img
  - article:
    - button "查看第 22 张 frame-021.jpg":
      - img "frame-021.jpg"
      - text: "22"
    - text: FRAME 22
    - button "标记保留 frame-021.jpg":
      - img
    - button "标记淘汰 frame-021.jpg":
      - img
  - article:
    - button "查看第 23 张 frame-022.jpg":
      - img "frame-022.jpg"
      - text: "23"
    - text: FRAME 23
    - button "标记保留 frame-022.jpg":
      - img
    - button "标记淘汰 frame-022.jpg":
      - img
  - article:
    - button "查看第 24 张 frame-023.jpg":
      - img "frame-023.jpg"
      - text: "24"
    - text: FRAME 24
    - button "标记保留 frame-023.jpg":
      - img
    - button "标记淘汰 frame-023.jpg":
      - img
  - strong: Selected 03
  - text: 作品初选序列
  - button "查看入选 frame-000.jpg":
    - img "frame-000.jpg"
  - button "查看入选 frame-001.jpg":
    - img "frame-001.jpg"
  - button "查看入选 frame-002.jpg":
    - img "frame-002.jpg"
  - button "编辑序列 →"
- complementary "照片详情":
  - heading "照片详情" [level=2]
  - region "照片详情":
    - button "查看完整大图":
      - img "当前选中：frame-000.jpg"
      - img
    - text: FRAME 01
    - heading "frame-000.jpg" [level=3]
    - text: 已保留
    - paragraph: 480 × 320
    - group "人工星级":
      - text: 你的评分
      - button "人工评分 1 星": ★
      - button "人工评分 2 星": ★
      - button "人工评分 3 星": ★
      - button "人工评分 4 星": ★
      - button "人工评分 5 星": ★
      - button "清除人工评分" [disabled]: 清除
    - group "册页标记":
      - text: 册页（印刷）
      - button "跨页大图"
      - button "整页图"
      - text: 默认每页两张
    - button "◇ 保留作品"
    - button "× 淘汰作品"
    - button "恢复待定"
    - text: 照片备注
    - textbox "照片备注":
      - /placeholder: 构图、光线、拍摄背景…
    - button "保存备注" [disabled]
```

# Test source

```ts
  186 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);
  187 |   const original=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});
  188 |   await dialog.getByRole('button',{name:'添加页面',exact:true}).click();const pageEditor=page.getByRole('dialog',{name:'添加页面',exact:true});
  189 |   await pageEditor.getByRole('spinbutton',{name:'页面宽度（mm）',exact:true}).fill('180');await pageEditor.getByRole('spinbutton',{name:'页面高度（mm）',exact:true}).fill('240');await pageEditor.getByRole('spinbutton',{name:'页边距（mm）',exact:true}).fill('12');await pageEditor.getByRole('button',{name:'添加页面',exact:true}).click();await expect(pageEditor).toHaveCount(0);
  190 |   const inserted=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const newPage=inserted.layout.pages[1];expect(inserted.layout.pages).toHaveLength(5);expect(newPage.widthMm).toBe(180);await expect(dialog.getByRole('combobox',{name:'选择页面',exact:true})).toHaveValue(newPage.id);
  191 |   await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('夜行\n在城市的间隙停留');
  192 |   await editor.getByRole('spinbutton',{name:'字号（pt）',exact:true}).fill('0');await expect(editor.getByRole('button',{name:'保存调整',exact:true})).toBeDisabled();expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).revision).toBe(inserted.revision);
  193 |   await editor.getByRole('spinbutton',{name:'字号（pt）',exact:true}).fill('26');await editor.getByRole('checkbox',{name:'加粗',exact:true}).check();await editor.getByRole('combobox',{name:'对齐',exact:true}).selectOption('center');
  194 |   const directory=evidenceDirectory('publication-editing');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'text-preview.png')});await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);
  195 |   await dialog.getByRole('button',{name:'添加页码',exact:true}).click();const numberEditor=page.getByRole('dialog',{name:'添加页码',exact:true});await numberEditor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(numberEditor).toHaveCount(0);
  196 |   let current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(current.layout.pages[1].texts).toHaveLength(2);const body=current.layout.pages[1].texts.find(t=>JSON.parse(t.contentJson).kind==='text')!;expect(JSON.parse(body.styleJson!).fontSizePt).toBe(26);
  197 |   await expect(dialog.locator('.publication-studio-preview .publication-text').filter({hasText:/^1$/})).toHaveCount(1);
  198 |   await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(original.layout.pages[1].id);await dialog.getByRole('button',{name:'向前移一页',exact:true}).click();await dialog.getByRole('group',{name:'页序预览',exact:true}).getByRole('button',{name:'保存顺序',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('页面顺序已保存。');
  199 |   current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(current.layout.pages[2].id).toBe(newPage.id);await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(newPage.id);await expect(dialog.locator('.publication-studio-preview .publication-text').filter({hasText:/^2$/})).toHaveCount(1);
  200 |   await dialog.locator('.publication-studio-preview .publication-text').filter({hasText:'夜行'}).dblclick();const editText=page.getByRole('dialog',{name:'编辑文字',exact:true});await editText.getByRole('textbox',{name:'文字内容',exact:true}).fill('<script>这是一段文字</script>');expect(await editText.locator('.publication-object-preview script').count()).toBe(0);await editText.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editText).toHaveCount(0);
  201 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('图文稿');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版本已保存。');
  202 |   const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const versions=await rpc<PublicationVersionInfo[]>('pub_versions',{projectId,publicationId,before:null});const savedId=versions.find(v=>v.label==='图文稿')!.id;
  203 |   await page.screenshot({path:resolve(directory,'page-and-text.png')});await dialog.getByRole('button',{name:'页面设置',exact:true}).click();const settings=page.getByRole('dialog',{name:'页面设置',exact:true});await settings.getByRole('button',{name:'移除页面',exact:true}).click();expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).revision).toBe(saved.revision);await settings.getByRole('button',{name:'确认移除页面',exact:true}).click();await expect(settings).toHaveCount(0);
  204 |   expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout.pages).toHaveLength(4);await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^图文稿/}).click();await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已恢复为新的版面版本。');
  205 |   expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);expect((await rpc<PublicationVersion>('pub_version',{projectId,publicationId,versionId:savedId})).layout).toEqual(saved.layout);
  206 |   await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  207 | });
  208 |
  209 | test('真实文字回执丢失：编辑保留、关闭后确认原请求、文字和历史不重复',async({page})=>{
  210 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);let lost=false;const commands:string[]=[];
  211 |   await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations[0].action.type==='AddText'){commands.push(data.args.request.commandId);if(!lost){lost=true;const reply=await(await route.fetch()).json();expect(reply.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
  212 |   await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('只保存一次');await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor.getByRole('button',{name:'保存调整',exact:true})).toBeDisabled();await expect(editor.getByRole('textbox',{name:'文字内容',exact:true})).toHaveValue('只保存一次');
  213 |   const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(saved.layout.pages[0].texts).toHaveLength(1);await editor.getByRole('button',{name:'关闭对话框',exact:true}).click();await expect(editor).toHaveCount(0);await dialog.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已确认保存结果');
  214 |   expect(commands).toHaveLength(2);expect(commands[1]).toBe(commands[0]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  215 | });
  216 |
  217 | test('真实照片编排：预览后排入、替换与移除、版本恢复、左右页编辑和重启',async({page})=>{
  218 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const project=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});
  219 |   const writes:unknown[]=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute')writes.push(data.args.request);await route.fallback();});
  220 |   await dialog.getByRole('button',{name:'添加照片',exact:true}).click();const picker=page.getByRole('dialog',{name:'添加照片',exact:true});await expect(picker.getByRole('checkbox').nth(0)).toBeDisabled();await picker.getByRole('checkbox').nth(1).check();await picker.getByRole('checkbox').nth(2).check();await expect(picker.getByText('已选 2 张 · 本次最多 16 张',{exact:true})).toBeVisible();expect(writes).toHaveLength(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(before.layout);
  221 |   const directory=evidenceDirectory('publication-frames');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'photo-layout-preview.png')});await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'photo-layout-compact.png')});await page.setViewportSize({width:1440,height:1000});
  222 |   await picker.getByRole('button',{name:'排入本页（2 张）',exact:true}).click();await expect(picker).toHaveCount(0);await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('照片已排入本页。');expect(writes).toHaveLength(1);
  223 |   const added=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(added.layout.pages[0].frames).toHaveLength(3);const selected=added.layout.pages[0].frames[0];const oldRef=added.previews.find(p=>p.frameId===selected.id)!.photoId;
  224 |   await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('button',{name:'替换照片',exact:true}).click();const replace=page.getByRole('dialog',{name:'替换照片',exact:true});await replace.getByRole('button',{name:/^全部照片 ·/}).click();await replace.getByRole('radio').nth(5).check();await replace.getByRole('button',{name:'替换照片',exact:true}).click();await expect(replace).toHaveCount(0);
  225 |   const replaced=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const frame=replaced.layout.pages[0].frames.find(f=>f.id===selected.id)!;expect(replaced.previews.find(p=>p.frameId===frame.id)!.photoId).not.toBe(oldRef);expect(frame.xMm).toBe(selected.xMm);expect(frame.widthMm).toBe(selected.widthMm);expect(frame.cropZoom).toBe(1);
  226 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('照片布局');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版本已保存。');
  227 |   await dialog.getByRole('button',{name:'移除照片框',exact:true}).click();const remove=page.getByRole('dialog',{name:'确认移除照片框',exact:true});expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(replaced.layout);await remove.getByRole('button',{name:'确认移除照片框',exact:true}).click();await expect(remove).toHaveCount(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout.pages[0].frames).toHaveLength(2);
  228 |   await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^照片布局/}).click();await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已恢复为新的版面版本。');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(replaced.layout);
  229 |   await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(replaced.layout.pages[2].id);await dialog.getByRole('button',{name:'跨页预览',exact:true}).click();const left=dialog.getByRole('figure',{name:'第 2 页',exact:true}),right=dialog.getByRole('figure',{name:'第 3 页',exact:true});await expect(left).toBeVisible();await expect(right).toBeVisible();await right.getByRole('button',{name:'选择照片框 1',exact:true}).click();await expect(dialog.getByRole('combobox',{name:'选择页面',exact:true})).toHaveValue(replaced.layout.pages[3].id);
  230 |   await dialog.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('50');await expect(left.getByRole('button',{name:'选择照片框 1',exact:true})).toBeDisabled();await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版面调整已保存。');
  231 |   const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(saved.layout.pages[3].frames[0].widthMm).toBe(50);expect(saved.layout.pages[2]).toEqual(replaced.layout.pages[2]);await page.screenshot({path:resolve(directory,'facing-pages.png')});
  232 |   const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  233 | });
  234 |
  235 | test('真实照片编排回执丢失：整组只确认原请求，不重复排版或照片框',async({page})=>{
  236 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);let lost=false;const requests:unknown[]=[];
  237 |   await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations.some((op:{action:{type:string}})=>op.action.type==='AddFrame')){requests.push(data.args.request);if(!lost){lost=true;const result=await(await route.fetch()).json();expect(result.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
  238 |   await dialog.getByRole('button',{name:'添加照片',exact:true}).click();const picker=page.getByRole('dialog',{name:'添加照片',exact:true});await picker.getByRole('checkbox').nth(1).check();await picker.getByRole('button',{name:'排入本页（1 张）',exact:true}).click();await expect(picker.getByRole('checkbox').nth(1)).toBeDisabled();await expect(picker.getByRole('button',{name:'排入本页（1 张）',exact:true})).toBeDisabled();
  239 |   const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(saved.layout.pages[0].frames).toHaveLength(2);const history=await rpc('pub_versions',{projectId,publicationId,before:null});await picker.getByRole('button',{name:'取消',exact:true}).click();await expect(picker).toHaveCount(0);await dialog.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已确认保存结果');expect(requests).toHaveLength(2);expect(requests[0]).toEqual(requests[1]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  240 | });
  241 |
  242 | test('真实母版：纯预览、批量关联、填空框、手工覆盖确认、更新与版本恢复',async({page})=>{
  243 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);
  244 |   const project=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});
  245 |   await dialog.getByRole('button',{name:'添加照片',exact:true}).click();let picker=page.getByRole('dialog',{name:'添加照片',exact:true});await picker.getByRole('checkbox').nth(1).check();await picker.getByRole('checkbox').nth(2).check();await picker.getByRole('button',{name:'排入本页（2 张）',exact:true}).click();await expect(picker).toHaveCount(0);
  246 |   await dialog.getByRole('button',{name:'添加页码',exact:true}).click();const text=page.getByRole('dialog',{name:'添加页码',exact:true});await text.getByRole('button',{name:'保存调整',exact:true}).click();await expect(text).toHaveCount(0);
  247 |   await dialog.getByRole('button',{name:'保存为母版',exact:true}).click();const save=page.getByRole('dialog',{name:'保存为母版',exact:true});await save.getByRole('textbox',{name:'母版名称',exact:true}).fill('三幅与页码');await save.getByRole('button',{name:'保存为母版',exact:true}).click();await expect(save).toHaveCount(0);
  248 |   let current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const target=current.layout.pages[1].id;const master=JSON.parse(current.layout.publication.specJson).focalroomPageMastersV1[0];expect(master.frames).toHaveLength(3);expect(master.texts).toHaveLength(1);
  249 |   await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(target);await dialog.getByRole('button',{name:'应用母版',exact:true}).click();let apply=page.getByRole('dialog',{name:'应用母版',exact:true});await apply.getByRole('button',{name:'选择全部内页',exact:true}).click();await expect(apply.getByRole('button',{name:'应用母版',exact:true})).toBeEnabled();expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(current.layout);
  250 |   const directory=evidenceDirectory('publication-masters');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'apply-preview.png')});await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'apply-preview-compact.png')});await page.setViewportSize({width:1440,height:1000});await apply.getByRole('button',{name:'应用母版',exact:true}).click();await expect(apply).toHaveCount(0);
  251 |   current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(current.layout.pages.slice(1).every(p=>p.frames.length===3)).toBe(true);const frames=current.layout.pages[1].frames;expect(frames.filter(f=>!f.assetId)).toHaveLength(2);
  252 |   await dialog.getByRole('button',{name:'添加照片',exact:true}).click();picker=page.getByRole('dialog',{name:'添加照片',exact:true});await expect(picker.getByText('照片将依次填入空照片框，不移动已有照片。',{exact:true})).toBeVisible();await picker.getByRole('checkbox').nth(1).check();await picker.getByRole('checkbox').nth(2).check();await picker.getByRole('button',{name:'排入本页（2 张）',exact:true}).click();await expect(picker).toHaveCount(0);
  253 |   current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(current.layout.pages[1].frames.every(f=>!!f.assetId)).toBe(true);expect(current.layout.pages[1].frames.map(f=>[f.id,f.xMm,f.yMm,f.widthMm,f.heightMm])).toEqual(frames.map(f=>[f.id,f.xMm,f.yMm,f.widthMm,f.heightMm]));expect(current.masterStates!.find(s=>s.pageId===target)!.locallyChanged).toBe(false);
  254 |   await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('40');await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.getByText('本页有手工调整',{exact:true})).toBeVisible();
  255 |   await dialog.getByRole('button',{name:'应用母版',exact:true}).click();apply=page.getByRole('dialog',{name:'应用母版',exact:true});await expect(apply.getByText(/1 页有手工调整/)).toBeVisible();await expect(apply.getByRole('button',{name:'应用母版',exact:true})).toBeDisabled();await apply.getByRole('checkbox',{name:'确认覆盖这些页面的版面调整及母版文字',exact:true}).check();await apply.getByRole('button',{name:'选择全部内页',exact:true}).click();await expect(apply.getByRole('checkbox',{name:'确认覆盖这些页面的版面调整及母版文字',exact:true})).not.toBeChecked();await expect(apply.getByRole('button',{name:'应用母版',exact:true})).toBeDisabled();await apply.getByRole('checkbox',{name:'确认覆盖这些页面的版面调整及母版文字',exact:true}).check();await page.screenshot({path:resolve(directory,'local-adjustment-confirmation.png')});await apply.getByRole('button',{name:'应用母版',exact:true}).click();await expect(apply).toHaveCount(0);
  256 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('母版已确认');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版本已保存。');const frozen=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});
  257 |   await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('50');await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await dialog.getByRole('button',{name:'更新母版',exact:true}).click();const update=page.getByRole('dialog',{name:'更新母版',exact:true});await update.getByRole('button',{name:'更新母版',exact:true}).click();await expect(update).toHaveCount(0);
  258 |   current=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(JSON.parse(current.layout.publication.specJson).focalroomPageMastersV1[0].revision).toBe(2);expect(current.layout.pages[0]).toEqual(frozen.layout.pages[0]);expect(current.masterStates!.find(s=>s.pageId===current.layout.pages[0].id)!.currentRevision).toBe(2);
  259 |   await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^母版已确认/}).click();await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('已恢复为新的版面版本。');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(frozen.layout);
  260 |   const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(frozen.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  261 | });
  262 |
  263 | test('真实文字排版检查：截断提示随实际字体与尺寸变化，修改前不写入',async({page})=>{
  264 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});
  265 |   await dialog.getByRole('button',{name:'添加文字',exact:true}).click();let editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('摄影作品与叙事节奏'.repeat(40));await editor.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('40');await editor.getByRole('spinbutton',{name:'高度（mm）',exact:true}).fill('2');await editor.getByRole('spinbutton',{name:'字号（pt）',exact:true}).fill('72');await expect(editor.locator('.publication-overflow-notice')).toHaveText('1 处文字未完整显示');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).revision).toBe(before.revision);const directory=evidenceDirectory('publication-masters');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'text-overflow.png')});await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);await expect(dialog.locator('.publication-studio-preview .publication-overflow-notice')).toBeVisible();
  266 |   await dialog.getByRole('button',{name:'选择文字框 1',exact:true}).click();await dialog.getByRole('button',{name:'编辑文字',exact:true}).click();editor=page.getByRole('dialog',{name:'编辑文字',exact:true});await editor.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('180');await editor.getByRole('spinbutton',{name:'高度（mm）',exact:true}).fill('150');await editor.getByRole('spinbutton',{name:'字号（pt）',exact:true}).fill('10');await expect(editor.locator('.publication-overflow-notice')).toHaveCount(0);await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);await expect(dialog.locator('.publication-studio-preview .publication-overflow-notice')).toHaveCount(0);
  267 |   const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});await dialog.getByRole('combobox',{name:'本页内容',exact:true}).selectOption('frame:'+saved.layout.pages[0].frames[0].id);await expect(dialog.locator('.publication-ppi')).toContainText('ppi');await page.screenshot({path:resolve(directory,'overlapping-content-selection.png')});expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  268 | });
  269 |
  270 | test('生成素材经插件回流代码：比较后明确使用，裁切和历史固定，恢复与重启保持',async({page})=>{
  271 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('spinbutton',{name:'放大倍数',exact:true}).fill('1.5');await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');
  272 |   const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),frame=before.layout.pages[0].frames[0];const project=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});await rpc('test_publication_versions',{projectId,publicationId,frameId:frame.id});
  273 |   const catalog=await rpc<{choices:Array<{renditionId:string;externalSoftware:string;source:{renditionSha256:string}}>} >('pub_photo_versions',{projectId,publicationId,frameId:frame.id,baseRevision:before.revision,cursor:null});const version=catalog.choices.find(v=>v.externalSoftware==='lightroom_classic')!;
  274 |   const writes:unknown[]=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute')writes.push(data.args.request);await route.fallback();});await dialog.getByRole('button',{name:'图像版本',exact:true}).click();let picker=page.getByRole('dialog',{name:'图像版本',exact:true});await picker.getByRole('radio',{name:/^Lightroom Classic/}).check();await expect(picker.getByRole('img',{name:'预览版本',exact:true})).toHaveAttribute('src',new RegExp(version.source.renditionSha256+'-320\\.jpg$'));await expect.poll(()=>picker.getByRole('img',{name:'预览版本',exact:true}).evaluate((node:HTMLImageElement)=>node.complete&&node.naturalWidth>0)).toBe(true);expect(await picker.textContent()).not.toMatch(/[a-f0-9]{64}/);expect(writes).toHaveLength(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(before.layout);
  275 |   const directory=evidenceDirectory('publication-photo-versions');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'version-comparison.png')});await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'version-comparison-compact.png')});await page.setViewportSize({width:1440,height:1000});await picker.getByRole('button',{name:'使用此版本',exact:true}).click();await expect(picker).toHaveCount(0);
  276 |   const changed=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),selected=changed.layout.pages[0].frames[0];expect(selected.renditionId).toBe(version.renditionId);expect({...selected,renditionId:null,source:null}).toEqual({...frame,renditionId:null,source:null});expect(writes).toHaveLength(1);
  277 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('Lightroom 方案');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版本已保存。');
  278 |   await dialog.getByRole('button',{name:'图像版本',exact:true}).click();picker=page.getByRole('dialog',{name:'图像版本',exact:true});await picker.getByRole('radio',{name:/^原片/}).check();await picker.getByRole('button',{name:'使用此版本',exact:true}).click();await expect(picker).toHaveCount(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout.pages[0].frames[0].renditionId).toBeNull();
  279 |   await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^Lightroom 方案/}).click();await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('已恢复为新的版面版本。');const restored=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(restored.layout).toEqual(changed.layout);expect(restored.previews.find(p=>p.frameId===frame.id)?.thumbnailSrc??JSON.stringify(restored.previews)).toContain(version.source.renditionSha256);await expect.poll(()=>dialog.locator('.publication-studio-preview .publication-frame img').first().evaluate((node:HTMLImageElement)=>node.complete&&node.naturalWidth>0)).toBe(true);
  280 |   const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(changed.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  281 | });
  282 |
  283 | test('真实照片版本回执丢失：不重复切换，返回后仅确认原请求',async({page})=>{
  284 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),frame=before.layout.pages[0].frames[0];await rpc('test_publication_versions',{projectId,publicationId,frameId:frame.id});let lost=false;const requests:unknown[]=[];
  285 |   await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations[0].action.type==='UpdateFrame'){requests.push(data.args.request);if(!lost){lost=true;const reply=await(await route.fetch()).json();expect(reply.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
> 286 |   await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('button',{name:'图像版本',exact:true}).click();const picker=page.getByRole('dialog',{name:'图像版本',exact:true});await picker.getByRole('radio',{name:/^Capture One/}).check();await expect.poll(()=>picker.getByRole('img',{name:'预览版本',exact:true}).evaluate((node:HTMLImageElement)=>node.complete&&node.naturalWidth>0)).toBe(true);await picker.getByRole('button',{name:'使用此版本',exact:true}).click();await expect(picker.getByRole('button',{name:'使用此版本',exact:true})).toBeDisabled();await expect(picker.getByRole('radio',{name:/^原片/})).toBeDisabled();const saved=await rpc('pub_layout',{projectId,publicationId}),history=await rpc('pub_versions',{projectId,publicationId,before:null});await picker.getByRole('button',{name:'返回',exact:true}).click();await expect(picker).toHaveCount(0);await dialog.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('已确认保存结果');expect(requests).toHaveLength(2);expect(requests[0]).toEqual(requests[1]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
      |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      ^ Error: expect(locator).toBeDisabled() failed
  287 | });
  288 |
  289 | test('页面直接编辑：照片拖动取消与比例缩放、文字移动、保存恢复和重启',async({page})=>{
  290 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('spinbutton',{name:'宽度（mm）',exact:true}).fill('80');await dialog.getByRole('spinbutton',{name:'高度（mm）',exact:true}).fill('100');await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');
  291 |   const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),original=before.layout.pages[0].frames[0];const project=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});await dialog.getByRole('combobox',{name:'页面缩放',exact:true}).selectOption('75');const physicalPage=(await dialog.locator('.publication-studio-preview .publication-paper').boundingBox())!;expect(physicalPage.width).toBeCloseTo(before.layout.pages[0].widthMm*96/25.4*.75,0);const writes:Array<{operations:Array<{action:{type:string}}>}>=[];await page.route(base+'/rpc',async route=>{const value=route.request().postDataJSON();if(value.command==='pub_execute')writes.push(value.args.request);await route.fallback();});
  292 |   let box=(await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).boundingBox())!;await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+24,box.y+box.height/2+14,{steps:5});await expect(dialog.getByText('尚未保存',{exact:true})).toBeVisible();expect(writes).toHaveLength(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(before.layout);await page.keyboard.press('Escape');await page.mouse.up();await expect(dialog).toBeVisible();await expect(dialog.getByRole('spinbutton',{name:'左侧位置（mm）',exact:true})).toHaveValue(String(original.xMm));await expect(dialog.getByRole('button',{name:'保存调整',exact:true})).toBeDisabled();
  293 |   box=(await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).boundingBox())!;await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+24,box.y+box.height/2+14,{steps:5});await page.mouse.up();await expect(dialog.getByRole('combobox',{name:'选择页面',exact:true})).toBeDisabled();await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const moved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),movedFrame=moved.layout.pages[0].frames[0];expect(movedFrame.xMm).toBeGreaterThan(original.xMm);expect(movedFrame.xMm).toBeCloseTo(original.xMm+24/physicalPage.width*before.layout.pages[0].widthMm,1);expect(movedFrame.yMm).toBeGreaterThan(original.yMm);expect({...movedFrame,xMm:original.xMm,yMm:original.yMm}).toEqual(original);expect(writes).toHaveLength(1);
  294 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('拖动排版');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版本已保存。');const corner=(await dialog.getByRole('button',{name:'调整右下角',exact:true}).boundingBox())!;await page.keyboard.down('Shift');await page.mouse.move(corner.x+corner.width/2,corner.y+corner.height/2);await page.mouse.down();await page.mouse.move(corner.x+corner.width/2+24,corner.y+corner.height/2+10,{steps:5});await page.mouse.up();await page.keyboard.up('Shift');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(moved.layout);await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const resized=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(resized.layout.pages[0].frames[0].widthMm).toBeGreaterThan(80);expect(resized.layout.pages[0].frames[0].widthMm/resized.layout.pages[0].frames[0].heightMm).toBeCloseTo(.8,4);
  295 |   await dialog.getByRole('combobox',{name:'页面缩放',exact:true}).selectOption('fit');const directory=evidenceDirectory('publication-direct');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'frame-handles.png')});await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^拖动排版/}).click();await expect(dialog.getByRole('button',{name:'调整右下角',exact:true})).toHaveCount(0);await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('已恢复为新的版面版本。');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(moved.layout);
  296 |   await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('第三章 · 傍晚的光');await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);const textBefore=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),text=textBefore.layout.pages[0].texts[0];await dialog.getByRole('button',{name:'选择文字框 1',exact:true}).click();await expect(page.getByRole('dialog',{name:'编辑文字',exact:true})).toHaveCount(0);await page.keyboard.press('ArrowRight');await expect(dialog.getByRole('spinbutton',{name:'左侧位置（mm）',exact:true})).toHaveValue(String(text.xMm+1));await dialog.getByRole('button',{name:'放弃调整',exact:true}).click();
  297 |   box=(await dialog.getByRole('button',{name:'选择文字框 1',exact:true}).boundingBox())!;await page.mouse.move(box.x+box.width*.3,box.y+box.height*.5);await page.mouse.down();await page.mouse.move(box.x+box.width*.3-14,box.y+box.height*.5+18,{steps:5});await page.mouse.up();expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(textBefore.layout);await page.screenshot({path:resolve(directory,'text-position-preview.png')});await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),changed=saved.layout.pages[0].texts[0];expect(changed.xMm).toBeLessThan(text.xMm);expect(changed.yMm).toBeGreaterThan(text.yMm);expect(changed.contentJson).toBe(text.contentJson);expect(changed.styleJson).toBe(text.styleJson);expect(writes[writes.length-1].operations[0].action.type).toBe('UpdateTextGeometry');
  298 |   await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'direct-editing-compact.png')});const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  299 | });
  300 |
  301 | test('文字位置保存回执丢失：新拖动冻结，重开后仅确认原请求',async({page})=>{
  302 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('仅移动这段说明');await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);let lost=false;const requests:unknown[]=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations[0].action.type==='UpdateTextGeometry'){requests.push(data.args.request);if(!lost){lost=true;const reply=await(await route.fetch()).json();expect(reply.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
  303 |   await dialog.getByRole('button',{name:'选择文字框 1',exact:true}).click();await page.keyboard.press('ArrowRight');await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.getByRole('button',{name:'确认上次保存',exact:true})).toBeVisible();await expect(dialog.getByRole('spinbutton',{name:'左侧位置（mm）',exact:true})).toBeDisabled();await expect(dialog.getByRole('button',{name:'调整右下角',exact:true})).toHaveCount(0);const saved=await rpc('pub_layout',{projectId,publicationId}),history=await rpc('pub_versions',{projectId,publicationId,before:null});await page.reload();await page.getByRole('button',{name:'出版物',exact:true}).click();await page.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(page.getByRole('button',{name:'确认上次保存',exact:true})).toHaveCount(0);expect(requests).toHaveLength(2);expect(requests[0]).toEqual(requests[1]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  304 | });
  305 |
  306 | test('混合多选编排：等距对齐、整组移动取消与缩放、一次保存和版本恢复',async({page})=>{
  307 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await dialog.getByRole('button',{name:'添加照片',exact:true}).click();const picker=page.getByRole('dialog',{name:'添加照片',exact:true});await picker.getByRole('checkbox').nth(1).check();await picker.getByRole('checkbox').nth(2).check();await picker.getByRole('button',{name:'排入本页（2 张）',exact:true}).click();await expect(picker).toHaveCount(0);
  308 |   for(const [number,x,y,width,height]of [[1,20,30,45,60],[2,110,110,40,50]]){await dialog.getByRole('combobox',{name:'本页内容',exact:true}).selectOption({label:'选择照片框 '+number});for(const [name,value]of [['宽度（mm）',width],['高度（mm）',height],['左侧位置（mm）',x],['顶部位置（mm）',y]] as const)await dialog.getByRole('spinbutton',{name,exact:true}).fill(String(value));await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');}
  309 |   await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('章节开场');for(const [name,value]of [['左侧位置（mm）',60],['顶部位置（mm）',210],['宽度（mm）',40],['高度（mm）',20]] as const)await editor.getByRole('spinbutton',{name,exact:true}).fill(String(value));await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);
  310 |   const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),original=before.layout.pages[0],project=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});const batches:Array<{operations:Array<{action:{type:string;data:Record<string,unknown>}}>}>=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations.length>1)batches.push(data.args.request);await route.fallback();});
  311 |   const tools=dialog.getByRole('region',{name:'多选编排',exact:true});await tools.locator('summary').click();const objectList=tools.getByRole('group',{name:'本页对象',exact:true});for(const name of ['选择照片框 1','选择照片框 2','选择文字框 1'])await objectList.getByRole('checkbox',{name,exact:true}).check();await expect(tools.getByText('已选 3 项',{exact:true})).toBeVisible();await tools.getByRole('combobox',{name:'对齐参照',exact:true}).selectOption('page');await tools.getByRole('button',{name:'横向等距',exact:true}).click();await tools.getByRole('button',{name:'顶对齐',exact:true}).click();expect(batches).toHaveLength(0);expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(before.layout);
  312 |   const directory=evidenceDirectory('publication-arrangement');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'align-and-distribute-preview.png')});await tools.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const aligned=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(aligned.revision).toBe(before.revision+1);expect(batches).toHaveLength(1);expect(batches[0].operations.map(op=>op.action.type)).toEqual(['UpdateFrame','UpdateFrame','UpdateTextGeometry']);expect(aligned.layout.pages[0].frames[0].xMm).toBe(15);expect(aligned.layout.pages[0].frames[1].xMm).toBe(155);expect(aligned.layout.pages[0].texts[0].xMm).toBe(87.5);expect(aligned.layout.pages[0].frames[2]).toEqual(original.frames[2]);await expect(tools.getByText('已选 3 项',{exact:true})).toBeVisible();
  313 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('整组编排');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版本已保存。');await expect(dialog.getByRole('button',{name:'选择照片框 1',exact:true})).toBeEnabled();const stablePaper=(await dialog.locator('.publication-studio-paper .publication-paper').boundingBox())!;let point=await publicationVisibleSelectionPoint(dialog);await page.mouse.move(point.x,point.y);await page.mouse.down();await page.mouse.move(point.x+10,point.y+10,{steps:4});await expect(dialog.getByText('尚未保存',{exact:true})).toBeVisible();expect((await dialog.locator('.publication-studio-paper .publication-paper').boundingBox())!).toEqual(stablePaper);await page.keyboard.press('Escape');await page.mouse.up();await expect(tools.getByText('已选 3 项',{exact:true})).toBeVisible();await expect(tools.getByRole('button',{name:'保存调整',exact:true})).toBeDisabled();expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(aligned.layout);
  314 |   point=await publicationVisibleSelectionPoint(dialog);await page.mouse.move(point.x,point.y);await page.mouse.down();await page.mouse.move(point.x+12,point.y+14,{steps:4});await page.mouse.up();await page.screenshot({path:resolve(directory,'group-move-preview.png')});await tools.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const moved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),dx=moved.layout.pages[0].frames[0].xMm-aligned.layout.pages[0].frames[0].xMm;expect(dx).toBeGreaterThan(0);expect(moved.layout.pages[0].frames[1].xMm-aligned.layout.pages[0].frames[1].xMm).toBeCloseTo(dx,6);expect(moved.layout.pages[0].texts[0].xMm-aligned.layout.pages[0].texts[0].xMm).toBeCloseTo(dx,6);expect(moved.layout.pages[0].frames[2]).toEqual(original.frames[2]);
  315 |   const corner=(await dialog.getByRole('button',{name:'调整右下角',exact:true}).boundingBox())!;await page.keyboard.down('Shift');await page.mouse.move(corner.x+corner.width/2,corner.y+corner.height/2);await page.mouse.down();await page.mouse.move(corner.x+corner.width/2-20,corner.y+corner.height/2-10,{steps:4});await page.mouse.up();await page.keyboard.up('Shift');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(moved.layout);await tools.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('版面调整已保存。');const resized=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),scale=resized.layout.pages[0].frames[0].widthMm/moved.layout.pages[0].frames[0].widthMm;expect(scale).toBeLessThan(1);expect(resized.layout.pages[0].texts[0].widthMm/moved.layout.pages[0].texts[0].widthMm).toBeCloseTo(scale,5);expect(resized.layout.pages[0].texts[0].contentJson).toBe(original.texts[0].contentJson);expect(resized.layout.pages[0].texts[0].styleJson).toBe(original.texts[0].styleJson);expect(resized.layout.pages[0].frames[0].cropZoom).toBe(original.frames[0].cropZoom);expect(batches).toHaveLength(3);expect(batches.every(batch=>batch.operations.length===3)).toBe(true);await page.screenshot({path:resolve(directory,'group-resized.png')});await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'group-arrangement-compact.png')});await page.setViewportSize({width:1440,height:1000});
  316 |   await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^整组编排/}).click();await expect(dialog.getByRole('button',{name:'调整右下角',exact:true})).toHaveCount(0);await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.locator('.publication-studio-notice')).toHaveText('已恢复为新的版面版本。');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(aligned.layout);const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(aligned.layout);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  317 | });
  318 |
  319 | test('混合多选保存丢回执：冻结新操作，重开仅确认原整组命令',async({page})=>{
  320 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await dialog.getByRole('button',{name:'添加文字',exact:true}).click();const editor=page.getByRole('dialog',{name:'添加文字',exact:true});await editor.getByRole('textbox',{name:'文字内容',exact:true}).fill('同照片一起移动');await editor.getByRole('button',{name:'保存调整',exact:true}).click();await expect(editor).toHaveCount(0);let lost=false;const requests:unknown[]=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute'&&data.args.request.operations.length===2){requests.push(data.args.request);if(!lost){lost=true;const result=await(await route.fetch()).json();expect(result.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
  321 |   await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await page.keyboard.down('Shift');await dialog.getByRole('button',{name:'选择文字框 1',exact:true}).click();await page.keyboard.up('Shift');await page.keyboard.press('ArrowRight');const tools=dialog.getByRole('region',{name:'多选编排',exact:true});await tools.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.getByRole('button',{name:'确认上次保存',exact:true})).toBeVisible();await expect(tools.getByRole('button',{name:'保存调整',exact:true})).toBeDisabled();await expect(dialog.getByRole('button',{name:'调整右下角',exact:true})).toHaveCount(0);const saved=await rpc('pub_layout',{projectId,publicationId}),history=await rpc('pub_versions',{projectId,publicationId,before:null});await page.reload();await page.getByRole('button',{name:'出版物',exact:true}).click();await page.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(page.getByRole('button',{name:'确认上次保存',exact:true})).toHaveCount(0);expect(requests).toHaveLength(2);expect(requests[0]).toEqual(requests[1]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  322 | });
  323 |
  324 | test('素材与页序：真实拖入只新增、空框键盘填入、拖动与键盘排序预览、版本恢复',async({page})=>{
  325 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);await page.emulateMedia({reducedMotion:'no-preference'});const initial=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),project=await rpc<{photos:Array<{id:string;name:string}>;sequence:string[]}>('get_project',{projectId});const photo=project.photos[7],fillPhoto=project.photos[8];const writes:Array<{operations:Array<{action:{type:string;data:Record<string,unknown>}}>}>=[];await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON();if(data.command==='pub_execute')writes.push(data.args.request);await route.fallback();});
  326 |   const paper=dialog.locator('.publication-studio-paper .publication-paper');const foreign=await page.evaluateHandle(()=>{const data=new DataTransfer();data.setData('application/x-focalroom-photo','foreign-workspace');data.setData('text/uri-list','https://invalid.example/photo.jpg');return data;});await paper.dispatchEvent('drop',{dataTransfer:foreign});await foreign.dispose();await expect(dialog.getByRole('group',{name:'照片放置预览',exact:true})).toHaveCount(0);expect(writes).toHaveLength(0);
  327 |   await dialog.getByRole('tab',{name:'项目照片',exact:true}).click();await dialog.getByRole('combobox',{name:'照片来源',exact:true}).selectOption('all');await dialog.getByRole('searchbox',{name:'搜索照片名称',exact:true}).fill(photo.name);const material=dialog.getByRole('button',{name:'放置 '+photo.name,exact:true});await expect(material).toBeEnabled();let box=(await paper.boundingBox())!;await material.hover();await page.mouse.down();await page.mouse.move(box.x+box.width*.7,box.y+box.height*.7,{steps:8});await expect(paper).toHaveClass(/is-photo-drop-target/);await page.keyboard.press('Escape');await page.mouse.up();await expect(dialog).toBeVisible();await expect(dialog.getByRole('group',{name:'照片放置预览',exact:true})).toHaveCount(0);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(initial);await material.dragTo(paper,{targetPosition:{x:box.width*.75,y:box.height*.72}});await expect(dialog.getByRole('group',{name:'照片放置预览',exact:true})).toBeVisible();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(initial);expect(writes).toHaveLength(0);const previewBox=(await paper.locator('.publication-frame').last().boundingBox())!,paperBox=(await paper.boundingBox())!;expect(previewBox.y).toBeGreaterThan(paperBox.y+paperBox.height*.4);const directory=evidenceDirectory('publication-drag');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'material-drop-preview.png')});await dialog.getByRole('button',{name:'取消放置',exact:true}).click();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(initial);
  328 |   box=(await paper.boundingBox())!;await material.dragTo(paper,{targetPosition:{x:box.width*.65,y:box.height*.65}});await dialog.getByRole('button',{name:'确认放置',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('照片已排入本页。');const placed=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});const addition=writes.find(value=>value.operations[0].action.type==='AddFrame')!;expect(addition.operations).toHaveLength(1);expect(addition.operations[0].action.data).toMatchObject({photoId:photo.id,renditionId:null,pageId:initial.layout.pages[0].id});const addedId=String(addition.operations[0].action.data.frameId);expect(placed.layout.pages[0].frames.filter(frame=>frame.id!==addedId)).toEqual(initial.layout.pages[0].frames);expect(placed.layout.pages.slice(1)).toEqual(initial.layout.pages.slice(1));await expect(material).toBeDisabled();
  329 |   await dialog.getByRole('button',{name:'保存为母版',exact:true}).click();const master=page.getByRole('dialog',{name:'保存为母版',exact:true});await master.getByRole('textbox',{name:'母版名称',exact:true}).fill('照片落位');await master.getByRole('button',{name:'保存为母版',exact:true}).click();await expect(master).toHaveCount(0);await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(initial.layout.pages[1].id);await dialog.getByRole('button',{name:'应用母版',exact:true}).click();const apply=page.getByRole('dialog',{name:'应用母版',exact:true});await apply.getByRole('button',{name:'应用母版',exact:true}).click();await expect(apply).toHaveCount(0);const blank=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),empty=blank.layout.pages[1].frames.find(frame=>!frame.assetId)!;expect(empty).toBeTruthy();await dialog.getByRole('tab',{name:'项目照片',exact:true}).click();await dialog.getByRole('combobox',{name:'照片来源',exact:true}).selectOption('all');await dialog.getByRole('searchbox',{name:'搜索照片名称',exact:true}).fill(fillPhoto.name);const fill=dialog.getByRole('button',{name:'放置 '+fillPhoto.name,exact:true});await fill.focus();await page.keyboard.press('Enter');await expect(dialog.getByRole('group',{name:'照片放置预览',exact:true})).toBeVisible();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(blank);await dialog.getByRole('button',{name:'确认放置',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('照片已排入本页。');const filled=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),filledFrame=filled.layout.pages[1].frames.find(frame=>frame.id===empty.id)!;expect(filledFrame.assetId).toBeTruthy();expect({...filledFrame,assetId:null,source:null}).toEqual({...empty,source:null});expect(filled.layout.pages[1].frames.filter(frame=>frame.id!==empty.id)).toEqual(blank.layout.pages[1].frames.filter(frame=>frame.id!==empty.id));expect(writes.filter(value=>value.operations[0].action.type==='SetFramePhoto')[0].operations[0].action.data).toEqual({frameId:empty.id,photoId:fillPhoto.id,renditionId:null});
  330 |   await dialog.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(initial.layout.pages[2].id);await dialog.getByRole('button',{name:'跨页预览',exact:true}).click();const spreadPhoto=project.photos[9];await dialog.getByRole('searchbox',{name:'搜索照片名称',exact:true}).fill(spreadPhoto.name);const rightPaper=dialog.getByRole('figure',{name:'第 3 页',exact:true}).locator('.publication-paper'),rightBox=(await rightPaper.boundingBox())!;await dialog.getByRole('button',{name:'放置 '+spreadPhoto.name,exact:true}).dragTo(rightPaper,{targetPosition:{x:rightBox.width*.7,y:rightBox.height*.7}});await expect(dialog.getByRole('group',{name:'照片放置预览',exact:true})).toBeVisible();await expect(dialog.getByRole('combobox',{name:'选择页面',exact:true})).toHaveValue(initial.layout.pages[3].id);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(filled);await dialog.getByRole('button',{name:'确认放置',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('照片已排入本页。');const spreadPlaced=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(spreadPlaced.layout.pages[2]).toEqual(filled.layout.pages[2]);expect(spreadPlaced.layout.pages[3].frames.length).toBe(filled.layout.pages[3].frames.length+1);expect(writes.filter(value=>value.operations[0].action.type==='AddFrame')[1].operations[0].action.data).toMatchObject({pageId:initial.layout.pages[3].id,photoId:spreadPhoto.id});await page.screenshot({path:resolve(directory,'facing-page-placement.png')});await dialog.getByRole('button',{name:'单页预览',exact:true}).click();
  331 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('拖入与填框');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版本已保存。');const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});await dialog.getByRole('tab',{name:'页面',exact:true}).click();const source=dialog.getByRole('button',{name:'拖动排序 第 3 页',exact:true}),target=dialog.getByRole('button',{name:'第 1 页',exact:true});await source.hover();const targetBox=(await target.boundingBox())!;await page.mouse.down();await page.mouse.move(targetBox.x+targetBox.width/2,targetBox.y+targetBox.height/2,{steps:8});await page.mouse.up();await expect(dialog.getByRole('group',{name:'页序预览',exact:true})).toBeVisible();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(writes.filter(value=>value.operations[0].action.type==='ReorderPages')).toHaveLength(0);await page.screenshot({path:resolve(directory,'page-order-preview.png')});await dialog.getByRole('button',{name:'保存顺序',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('页面顺序已保存。');const ordered=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(ordered.layout.pages.map(item=>item.id)).toEqual([saved.layout.pages[0].id,saved.layout.pages[3].id,saved.layout.pages[1].id,saved.layout.pages[2].id]);for(const item of ordered.layout.pages){const old=saved.layout.pages.find(value=>value.id===item.id)!;expect({...item,pageNumber:old.pageNumber,side:old.side}).toEqual(old);}expect(writes.filter(value=>value.operations[0].action.type==='ReorderPages')).toHaveLength(1);
  332 |   await liftPublicationPage(page,dialog);await page.keyboard.press('ArrowDown');await expect(dialog.locator('[id^=DndLiveRegion]')).toHaveText('移动至 第 3 页。');await page.keyboard.press('Space');await expect(dialog.getByRole('group',{name:'页序预览',exact:true})).toBeVisible();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(ordered);await dialog.getByRole('button',{name:'放弃排序',exact:true}).click();await expect(dialog.getByRole('group',{name:'页序预览',exact:true})).toHaveCount(0);await liftPublicationPage(page,dialog);await page.keyboard.press('ArrowDown');await expect(dialog.locator('[id^=DndLiveRegion]')).toHaveText('移动至 第 3 页。');await page.keyboard.press('Escape');await expect(dialog.locator('[id^=DndLiveRegion]')).toHaveText('已取消排序。');await expect(dialog.getByRole('group',{name:'页序预览',exact:true})).toHaveCount(0);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(ordered);
  333 |   await page.setViewportSize({width:1024,height:768});await page.screenshot({path:resolve(directory,'page-sort-compact.png')});await page.setViewportSize({width:1440,height:1000});await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^拖入与填框/}).click();await expect(dialog.getByRole('button',{name:/^拖动排序/})).toHaveCount(0);await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已恢复为新的版面版本。');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(saved.layout);const unchanged=await rpc<{photos:unknown[];sequence:string[]}>('get_project',{projectId});expect(unchanged.photos).toEqual(project.photos);expect(unchanged.sequence).toEqual(project.sequence);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  334 | });
  335 |
  336 | test('素材与页序丢回执：重开只确认原照片放置和原完整页序，不重复历史',async({page})=>{
  337 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);const initial=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),project=await rpc<{photos:Array<{id:string;name:string}>}>('get_project',{projectId}),photo=project.photos[7];const requests:Record<string,unknown[]>={AddFrame:[],ReorderPages:[]};await page.route(base+'/rpc',async route=>{const data=route.request().postDataJSON(),kind=data.args?.request?.operations?.[0]?.action?.type;if(data.command==='pub_execute'&&requests[kind]){requests[kind].push(data.args.request);if(requests[kind].length===1){const reply=await(await route.fetch()).json();expect(reply.ok).toBe(true);await route.fulfill({json:{ok:false,error:{code:'DATABASE',message:'lost after commit'}}});return;}}await route.fallback();});
  338 |   await dialog.getByRole('tab',{name:'项目照片',exact:true}).click();await dialog.getByRole('combobox',{name:'照片来源',exact:true}).selectOption('all');await dialog.getByRole('searchbox',{name:'搜索照片名称',exact:true}).fill(photo.name);await dialog.getByRole('button',{name:'放置 '+photo.name,exact:true}).click();await dialog.getByRole('button',{name:'确认放置',exact:true}).click();await expect(dialog.getByRole('button',{name:'确认上次保存',exact:true})).toBeVisible();await expect(dialog.getByRole('button',{name:'确认放置',exact:true})).toBeDisabled();let saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),history=await rpc('pub_versions',{projectId,publicationId,before:null});expect(saved.layout.pages[0].frames.length).toBe(initial.layout.pages[0].frames.length+1);await page.reload();await page.getByRole('button',{name:'出版物',exact:true}).click();await page.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(page.getByRole('button',{name:'确认上次保存',exact:true})).toHaveCount(0);expect(requests.AddFrame).toHaveLength(2);expect(requests.AddFrame[1]).toEqual(requests.AddFrame[0]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);
  339 |   await page.getByRole('button',{name:'版面与版本',exact:true}).click();const studio=page.getByRole('dialog',{name:'版面与版本',exact:true});await studio.getByRole('combobox',{name:'选择页面',exact:true}).selectOption(initial.layout.pages[3].id);await studio.getByRole('button',{name:'向前移一页',exact:true}).click();await studio.getByRole('button',{name:'保存顺序',exact:true}).click();await expect(studio.getByRole('button',{name:'确认上次保存',exact:true})).toBeVisible();await expect(studio.getByRole('button',{name:'保存顺序',exact:true})).toBeDisabled();saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});history=await rpc('pub_versions',{projectId,publicationId,before:null});await page.reload();await page.getByRole('button',{name:'出版物',exact:true}).click();await page.getByRole('button',{name:'确认上次保存',exact:true}).click();await expect(page.getByRole('button',{name:'确认上次保存',exact:true})).toHaveCount(0);expect(requests.ReorderPages).toHaveLength(2);expect(requests.ReorderPages[1]).toEqual(requests.ReorderPages[0]);expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(saved);expect(await rpc('pub_versions',{projectId,publicationId,before:null})).toEqual(history);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  340 | });
  341 |
  342 | test('页序键盘：减少动态效果时仍能拿起、移动、预览及保存完整页序',async({page})=>{
  343 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});await liftPublicationPage(page,dialog);await expect(dialog.locator('.publication-page-item.is-dragging')).toHaveCSS('transition-duration','0s');await page.keyboard.press('ArrowDown');await expect(dialog.locator('[id^=DndLiveRegion]')).toHaveText('移动至 第 3 页。');await page.keyboard.press('Space');await expect(dialog.getByRole('group',{name:'页序预览',exact:true})).toBeVisible();expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(before);await dialog.getByRole('button',{name:'保存顺序',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('页面顺序已保存。');const saved=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(saved.layout.pages.map(item=>item.id)).toEqual([before.layout.pages[0].id,before.layout.pages[2].id,before.layout.pages[3].id,before.layout.pages[1].id]);for(const item of saved.layout.pages){const old=before.layout.pages.find(value=>value.id===item.id)!;expect({...item,pageNumber:old.pageNumber,side:old.side}).toEqual(old);}expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  344 | });
  345 |
  346 | test('连续跨页照片：明确预览、右页整体拖动、图像版本与历史恢复保持单一来源',async({page})=>{
  347 |   const {projectId,publicationId,dialog,hashes,errors,outside}=await prepare(page);
  348 |   await page.emulateMedia({reducedMotion:'no-preference'});
  349 |   const before=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId}),left=before.layout.pages[2],right=before.layout.pages[3],source=right.frames[0];
  350 |   const choose=dialog.getByRole('combobox',{name:'选择页面',exact:true});
  351 |   // 留出完整跨页画面，通过真实界面明确移除左页原有照片。
  352 |   await choose.selectOption(left.id);await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();await dialog.getByRole('button',{name:'移除照片框',exact:true}).click();
  353 |   await page.getByRole('dialog',{name:'确认移除照片框',exact:true}).getByRole('button',{name:'确认移除照片框',exact:true}).click();
  354 |   await expect(page.getByRole('dialog',{name:'确认移除照片框',exact:true})).toHaveCount(0);
  355 |   await choose.selectOption(right.id);await dialog.getByRole('button',{name:'选择照片框 1',exact:true}).click();
  356 |   await dialog.getByRole('button',{name:'展开为跨页照片',exact:true}).click();const expansion=page.getByRole('dialog',{name:'展开为跨页照片',exact:true});
  357 |   const stage=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(stage.layout.schemaVersion).toBe(1);await expect(expansion.getByRole('button',{name:'跨页照片 1',exact:true})).toHaveCount(2);
  358 |   await expect.poll(()=>expansion.locator('.publication-continuous-image img').evaluateAll(nodes=>nodes.every(node=>(node as HTMLImageElement).complete&&(node as HTMLImageElement).naturalWidth>0))).toBe(true);
  359 |   const directory=evidenceDirectory('publication-continuous');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'spread-preview.png')});
  360 |   await expansion.getByRole('button',{name:'展开为跨页照片',exact:true}).click();await expect(expansion).toHaveCount(0);
  361 |   let linked=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(linked.layout.schemaVersion).toBe(2);expect(linked.layout.pages[3].frames).toEqual([]);
  362 |   let canonical=linked.layout.pages[2].frames.find(frame=>frame.id===source.id)!;expect(canonical.source).toEqual(source.source);expect(linked.layout.pages.flatMap(p=>p.frames).filter(frame=>frame.id===source.id)).toHaveLength(1);
  363 |   const papers=dialog.locator('.publication-studio-papers .publication-studio-paper');await expect(papers).toHaveCount(2);
  364 |   const leftPaper=papers.nth(0).locator('.publication-paper'),rightPaper=papers.nth(1).locator('.publication-paper');
  365 |   await expect.poll(async()=>{const a=await leftPaper.boundingBox(),b=await rightPaper.boundingBox();return !!a&&!!b&&Math.abs(a.x+a.width-b.x)<1&&Math.abs(a.y-b.y)<.1&&Math.abs(a.height-b.height)<.1;}).toBe(true);
  366 |   const rightHalf=rightPaper.getByRole('button',{name:'跨页照片 1',exact:true});await rightHalf.click();await expect(dialog.locator('.publication-studio-papers .publication-resize-handle')).toHaveCount(4);
  367 |   const bounds=(await rightPaper.boundingBox())!,photo=(await rightHalf.boundingBox())!,x=photo.x+photo.width/2,y=photo.y+photo.height/2;
  368 |   await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+bounds.width*5/right.widthMm,y+bounds.height*3/right.heightMm,{steps:8});await page.mouse.up();
  369 |   await expect(dialog.getByRole('spinbutton',{name:'左侧位置（mm）',exact:true})).toHaveValue('20');await expect(dialog.getByRole('spinbutton',{name:'顶部位置（mm）',exact:true})).toHaveValue('18');
  370 |   expect((await rpc<PublicationDocument>('pub_layout',{projectId,publicationId})).layout).toEqual(linked.layout);
  371 |   await dialog.getByRole('button',{name:'保存调整',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版面调整已保存。');
  372 |   linked=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});canonical=linked.layout.pages[2].frames.find(frame=>frame.id===source.id)!;expect(canonical.xMm).toBeCloseTo(20,3);expect(canonical.yMm).toBeCloseTo(18,3);
  373 |   await dialog.getByRole('textbox',{name:'版本名称',exact:true}).fill('连续跨页确认版');await dialog.getByRole('button',{name:'保存版本',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('版本已保存。');
  374 |   const frozen=(await rpc<PublicationVersionInfo[]>('pub_versions',{projectId,publicationId,before:null})).find(v=>v.label==='连续跨页确认版')!;
  375 |   await rpc('test_publication_versions',{projectId,publicationId,frameId:source.id});
  376 |   await dialog.getByRole('button',{name:'图像版本',exact:true}).click();const versions=page.getByRole('dialog',{name:'图像版本',exact:true});await versions.getByRole('radio',{name:/^Lightroom Classic/}).check();await expect.poll(()=>versions.getByRole('img',{name:'预览版本',exact:true}).evaluate((img:HTMLImageElement)=>img.complete&&img.naturalWidth>0)).toBe(true);await versions.getByRole('button',{name:'使用此版本',exact:true}).click();await expect(versions).toHaveCount(0);
  377 |   const changed=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(changed.layout.pages[2].frames.find(frame=>frame.id===source.id)!.renditionId).not.toBeNull();expect(changed.layout.pages[3].frames).toEqual([]);
  378 |   const sources=await dialog.locator('.publication-studio-papers .publication-continuous-image img').evaluateAll(nodes=>nodes.map(node=>(node as HTMLImageElement).src));expect(sources).toHaveLength(2);expect(sources[0]).toBe(sources[1]);
  379 |   await page.screenshot({path:resolve(directory,'spread-editing.png')});await page.setViewportSize({width:1024,height:768});await expect(rightHalf).toBeVisible();await page.screenshot({path:resolve(directory,'spread-compact.png')});await page.setViewportSize({width:1440,height:1000});
  380 |   await dialog.getByRole('tab',{name:'版本记录',exact:true}).click();await dialog.getByRole('button',{name:/^连续跨页确认版/}).click();await dialog.getByRole('button',{name:'与当前并排比较',exact:true}).click();await expect(dialog.locator('.publication-studio-papers .publication-continuous-image img')).toHaveCount(4);
  381 |   await dialog.getByRole('button',{name:'恢复此版本',exact:true}).click();await dialog.getByRole('group',{name:'确认恢复',exact:true}).getByRole('button',{name:'确认恢复',exact:true}).click();await expect(dialog.getByRole('status',{name:'版面状态',exact:true})).toHaveText('已恢复为新的版面版本。');
  382 |   const restored=await rpc<PublicationDocument>('pub_layout',{projectId,publicationId});expect(restored.layout).toEqual(linked.layout);expect((await rpc<PublicationVersion>('pub_version',{projectId,publicationId,versionId:frozen.id})).layout).toEqual(linked.layout);
  383 |   await dialog.getByRole('button',{name:'关闭对话框',exact:true}).click();await page.goto('about:blank');await rpc('test_process_restart');expect(await rpc('pub_layout',{projectId,publicationId})).toEqual(restored);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
  384 | });
  385 |
  386 | test('连续跨页的页面操作：配对插入、整组移动、开本联动、转回单页与移除提示',async({page})=>{
```
