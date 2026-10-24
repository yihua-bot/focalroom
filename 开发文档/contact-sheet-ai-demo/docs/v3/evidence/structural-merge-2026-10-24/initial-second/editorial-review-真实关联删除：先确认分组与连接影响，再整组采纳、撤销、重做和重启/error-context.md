# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: editorial-review.spec.ts >> 真实关联删除：先确认分组与连接影响，再整组采纳、撤销、重做和重启
- Location: e2e/editorial-review.spec.ts:186:1

# Error details

```
Error: expect(received).toEqual(expected) // deep equality

- Expected  - 1
+ Received  + 1

@@ -30,11 +30,11 @@
      "locked": false,
      "nodeType": "group",
      "renditionRef": null,
      "rotation": 0,
      "styleJson": null,
-     "updatedAt": "2026-10-24T12:53:12.982508+00:00",
+     "updatedAt": "2026-10-24T12:53:13.129518+00:00",
      "visible": true,
      "width": 480,
      "x": 20,
      "y": 20,
      "zOrder": 3,
```

# Test source

```ts
  112 |       lost = true; const result = await (await route.fetch()).json(); expect(result.ok).toBe(true);
  113 |       await route.fulfill({ json: { ok: false, error: { code: 'DATABASE', message: 'lost after commit', retriable: true } } });
  114 |     } else await route.fallback();
  115 |   });
  116 |   await page.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  117 |   await expect(page.getByText(/尚未确认画布操作结果/)).toBeVisible();
  118 |   await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  119 |   await page.getByRole('button', { name: /^确认上次操作/ }).click();
  120 |   await expect(page.getByRole('button', { name: /^确认上次操作/ })).toHaveCount(0);
  121 |   const result = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source }); expect(result.board.revision).toBe(original.board.revision + 1);
  122 |   expect(result.nodes.find(n => n.id === photo.sourceNodeId)?.x).toBe(photo.after!.x);
  123 | });
  124 | 
  125 | test('真实三方冲突、连续分批采纳与拒绝决定在进程重启后保留', async ({ page }) => {
  126 |   const { projectId, source, draft, comparison, original, errors, outside } = await prepare(page);
  127 |   await page.getByRole('button', { name: '关闭对话框', exact: true }).click();
  128 |   const photo = comparison.changes.find(c => c.nodeType === 'photo')!;
  129 |   await rpc('canvas_execute', { request: { schemaVersion: 3, commandId: randomUUID(), idempotencyKey: randomUUID(), projectId, boardId: source, baseRevision: original.board.revision,
  130 |     operations: [{ opId: randomUUID(), action: { type: 'MoveNodes', data: { boardId: source, nodeIds: [photo.sourceNodeId], dx: 15, dy: 0 } } }] } });
  131 |   await page.reload();
  132 |   await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  133 |   await page.getByRole('combobox', { name: '选择画布' }).selectOption(draft);
  134 |   await page.getByRole('button', { name: '审查改动', exact: true }).click();
  135 |   const dialog = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  136 |   const index = comparison.changes.indexOf(photo);
  137 |   await dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true }).check();
  138 |   await expect(dialog.getByRole('region', { name: '选择冲突处理方式' })).toBeVisible();
  139 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  140 |   const directory = evidenceDirectory('editorial-review'); await mkdir(directory, { recursive: true });
  141 |   await page.screenshot({ path: resolve(directory, 'attribute-conflict.png') });
  142 |   await dialog.getByRole('button', { name: '使用建议', exact: true }).click();
  143 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  144 |   await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  145 |   await expect(dialog.getByRole('checkbox', { name: `选择改动 ${index + 1}`, exact: true })).toBeDisabled();
  146 |   const secondIndex = comparison.changes.findIndex(c => c.kind === 'change' && c.nodeType === 'sticky');
  147 |   expect(secondIndex).toBeGreaterThanOrEqual(0);
  148 |   await dialog.getByRole('checkbox', { name: `选择改动 ${secondIndex + 1}`, exact: true }).check();
  149 |   await expect(dialog.getByRole('button', { name: '采纳选中改动', exact: true })).toBeEnabled();
  150 |   await dialog.getByRole('button', { name: '采纳选中改动', exact: true }).click();
  151 |   await expect(dialog.getByRole('checkbox', { name: `选择改动 ${secondIndex + 1}`, exact: true })).toBeDisabled();
  152 |   const lastIndex = comparison.changes.findIndex(c => c.kind === 'add' && c.nodeType === 'sticky');
  153 |   expect(lastIndex).toBeGreaterThanOrEqual(0);
  154 |   await dialog.getByRole('checkbox', { name: `选择改动 ${lastIndex + 1}`, exact: true }).check();
  155 |   await dialog.getByRole('button', { name: '不采纳', exact: true }).click();
  156 |   await expect(dialog.getByText(/未采纳/)).toBeVisible();
  157 |   await page.screenshot({ path: resolve(directory, 'successive-decisions.png') });
  158 |   const beforeRestart = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  159 |   expect(beforeRestart.board.revision).toBe(original.board.revision + 3);
  160 |   const decisions = await rpc<CanvasComparison>('canvas_comparison', { projectId, boardId: draft });
  161 |   expect(decisions.changes.filter(c => c.decision === 'accepted')).toHaveLength(2);
  162 |   expect(decisions.changes[lastIndex].decision).toBe('rejected');
  163 |   await page.goto('about:blank'); await rpc('test_process_restart');
  164 |   expect(await rpc('canvas_comparison', { projectId, boardId: draft })).toEqual(decisions);
  165 |   expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(beforeRestart);
  166 |   await page.goto('/#/project/' + projectId);
  167 |   await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  168 |   await page.getByRole('combobox', { name: '选择画布' }).selectOption(source);
  169 |   await page.getByRole('button', { name: '采纳历史', exact: true }).click();
  170 |   const history = page.getByRole('dialog', { name: '采纳历史', exact: true });
  171 |   await expect(history.getByRole('button', { name: '预览撤回', exact: true })).toHaveCount(2);
  172 |   await history.getByRole('button', { name: '预览撤回', exact: true }).last().click();
  173 |   await expect(history.getByRole('button', { name: '确认撤回', exact: true })).toBeEnabled();
  174 |   expect(await rpc('canvas_snapshot', { projectId, boardId: source })).toEqual(beforeRestart);
  175 |   await page.screenshot({ path: resolve(directory, 'historical-revert-preview.png') });
  176 |   await history.getByRole('button', { name: '确认撤回', exact: true }).click();
  177 |   await expect(history.getByText(/已撤回/)).toBeVisible();
  178 |   const reversed = await rpc<Snapshot>('canvas_snapshot', { projectId, boardId: source });
  179 |   expect(reversed.nodes.find(n => n.id === photo.sourceNodeId)!.x).toBe(photo.before!.x + 15);
  180 |   const second = comparison.changes[secondIndex];
  181 |   expect(reversed.nodes.find(n => n.id === second.sourceNodeId)!.x).toBe(beforeRestart.nodes.find(n => n.id === second.sourceNodeId)!.x);
  182 |   expect(reversed.board.revision).toBe(beforeRestart.board.revision + 1);
  183 |   expect(errors).toEqual([]); expect(outside).toEqual([]);
  184 | });
  185 | 
  186 | test('真实关联删除：先确认分组与连接影响，再整组采纳、撤销、重做和重启',async({page})=>{
  187 |   const {projectId,source,draft,comparison,original,errors,outside}=await prepare(page,false,'画布结构移除验收');
  188 |   await page.getByRole('button',{name:'关闭对话框',exact:true}).click();
  189 |   const photo=comparison.changes.find(c=>c.kind==='remove'&&c.nodeType==='photo')!;expect(photo).toBeTruthy();
  190 |   const sourcePhoto=photo.sourceNodeId!;const other=original.nodes.find(n=>n.id!==sourcePhoto)!;
  191 |   const groupId=randomUUID(),arrowId=randomUUID();
  192 |   await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:source,baseRevision:original.board.revision,operations:[
  193 |     {opId:groupId,action:{type:'GroupNodes',data:{boardId:source,nodeIds:[sourcePhoto,other.id],label:'保留的组合'}}},
  194 |     {opId:arrowId,action:{type:'AddArrow',data:{boardId:source,fromNodeId:sourcePhoto,toNodeId:other.id}}},
  195 |     {opId:randomUUID(),action:{type:'ResizeNode',data:{boardId:source,nodeId:sourcePhoto,width:300,height:210}}},
  196 |   ]}});
  197 |   const before=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});const hashes=await rpc('test_fixture_checksums');
  198 |   await page.reload();await page.getByRole('button',{name:'叙事画布',exact:true}).click();await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  199 |   const dialog=page.getByRole('dialog',{name:'智能编辑审稿',exact:true});
  200 |   await dialog.getByRole('checkbox',{name:`选择改动 ${comparison.changes.indexOf(photo)+1}`,exact:true}).check();
  201 |   const conflicts=dialog.getByRole('region',{name:'处理内容与关联变化'});await expect(conflicts).toBeVisible();
  202 |   await conflicts.getByText('一同调整的内容 · 2',{exact:true}).click();await expect(conflicts.getByText(/移除失去端点的连接线/)).toBeVisible();
  203 |   await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeDisabled();expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(before);
  204 |   const directory=evidenceDirectory('structural-merge');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'related-removal-confirmation.png')});
  205 |   await conflicts.getByRole('button',{name:'确认移除并整理关联',exact:true}).click();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeEnabled();
  206 |   expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(before);
  207 |   await dialog.getByRole('button',{name:'采纳选中改动',exact:true}).click();await dialog.getByRole('button',{name:'查看原画布',exact:true}).click();
  208 |   const applied=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});expect(applied.nodes.some(n=>n.id===sourcePhoto||n.id===arrowId)).toBe(false);expect(JSON.parse(applied.nodes.find(n=>n.id===groupId)!.childrenJson!)).toEqual([other.id]);
  209 |   await page.getByRole('button',{name:'撤销',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.length).toBe(before.nodes.length);
  210 |   expect((await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.find(n=>n.id===sourcePhoto)!.width).toBe(300);
  211 |   await page.getByRole('button',{name:'重做',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.length).toBe(applied.nodes.length);
> 212 |   await page.goto('about:blank');await rpc('test_process_restart');expect((await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes).toEqual(applied.nodes);expect(await rpc('test_fixture_checksums')).toEqual(hashes);expect(errors).toEqual([]);expect(outside).toEqual([]);
      |                                                                                                                                                      ^ Error: expect(received).toEqual(expected) // deep equality
  213 | });
  214 | 
  215 | test('真实原内容已移除：确认后恢复稳定身份，拒绝不会偷偷复原',async({page})=>{
  216 |   const {projectId,source,draft,comparison,original,errors,outside}=await prepare(page);
  217 |   await page.getByRole('button',{name:'关闭对话框',exact:true}).click();
  218 |   const photo=comparison.changes.find(c=>c.nodeType==='photo'&&c.kind==='change')!;
  219 |   await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:source,baseRevision:original.board.revision,operations:[{opId:randomUUID(),action:{type:'DeleteNode',data:{boardId:source,nodeId:photo.sourceNodeId}}}]}});
  220 |   const deleted=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});
  221 |   await page.reload();await page.getByRole('button',{name:'叙事画布',exact:true}).click();await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  222 |   const dialog=page.getByRole('dialog',{name:'智能编辑审稿',exact:true});const checkbox=dialog.getByRole('checkbox',{name:`选择改动 ${comparison.changes.indexOf(photo)+1}`,exact:true});await checkbox.check();
  223 |   const conflicts=dialog.getByRole('region',{name:'处理内容与关联变化'});await expect(conflicts.getByText(/原内容已移除/)).toBeVisible();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeDisabled();
  224 |   await conflicts.getByRole('button',{name:'保留当前，不采纳此项',exact:true}).click();await expect(dialog.locator('.editorial-change').filter({hasText:'未采纳'})).toBeVisible();expect(await rpc('canvas_snapshot',{projectId,boardId:source})).toEqual(deleted);
  225 |   await checkbox.check();await conflicts.getByRole('button',{name:'按建议恢复',exact:true}).click();await expect(dialog.getByRole('button',{name:'采纳选中改动',exact:true})).toBeEnabled();
  226 |   const directory=evidenceDirectory('structural-merge');await mkdir(directory,{recursive:true});await page.screenshot({path:resolve(directory,'restore-confirmation.png')});
  227 |   await dialog.getByRole('button',{name:'采纳选中改动',exact:true}).click();await dialog.getByRole('button',{name:'查看原画布',exact:true}).click();
  228 |   const restored=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source});const node=restored.nodes.find(n=>n.id===photo.sourceNodeId)!;expect(node.assetId).toBe(photo.before!.assetId);expect(node.x).toBe(photo.after!.x);expect(restored.nodes.length).toBe(original.nodes.length);
  229 |   await page.getByRole('button',{name:'撤销',exact:true}).click();await expect.poll(async()=>(await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).nodes.some(n=>n.id===photo.sourceNodeId)).toBe(false);expect(errors).toEqual([]);expect(outside).toEqual([]);
  230 | });
  231 | 
  232 | test('真实讨论与修订：批注回复解决重开、重启保留、用户发送后创建独立新版本', async ({ page }) => {
  233 |   const { projectId,source,draft,errors,outside } = await prepare(page,true);
  234 |   const parent = await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:draft});
  235 |   const official = await rpc('get_project',{projectId});
  236 |   const review = page.getByRole('dialog',{name:'智能编辑审稿',exact:true});
  237 |   const discussion = review.getByRole('region',{name:'批注与讨论'});
  238 |   await discussion.getByRole('textbox',{name:'写下修改意见'}).fill('保留开篇，中段需要观看距离的变化');
  239 |   await discussion.getByRole('button',{name:'保存批注',exact:true}).click();
  240 |   await expect(discussion.getByText('保留开篇，中段需要观看距离的变化',{exact:true})).toBeVisible();
  241 |   await discussion.getByRole('button',{name:'回复',exact:true}).click();
  242 |   await discussion.getByRole('textbox',{name:'回复内容'}).fill('请用空镜作为中段过渡');
  243 |   await discussion.getByRole('textbox',{name:'回复内容'}).locator('..').getByRole('button',{name:'保存批注',exact:true}).click();
  244 |   await expect(discussion.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  245 |   await discussion.getByRole('button',{name:'标记已解决'}).click();
  246 |   await expect(discussion.getByText('已解决',{exact:true})).toBeVisible();
  247 |   await discussion.getByRole('button',{name:'重新讨论'}).click();
  248 |   await expect(discussion.getByRole('button',{name:'回复',exact:true})).toBeEnabled();
  249 |   expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  250 |   expect(await rpc('get_project',{projectId})).toEqual(official);
  251 |   const directory = evidenceDirectory('discussion'); await mkdir(directory,{recursive:true});
  252 |   await discussion.scrollIntoViewIfNeeded(); await page.screenshot({path:resolve(directory,'discussion-and-reply.png')});
  253 |   await page.goto('about:blank'); await rpc('test_process_restart');
  254 |   await page.goto('/#/project/'+projectId); await page.getByRole('button',{name:'叙事画布',exact:true}).click();
  255 |   await page.getByRole('combobox',{name:'选择画布'}).selectOption(draft); await page.getByRole('button',{name:'审查改动',exact:true}).click();
  256 |   const restored = page.getByRole('region',{name:'批注与讨论'});
  257 |   await expect(restored.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  258 |   const prior = await rpc<{tasks:{id:string;state:string;boardDrafts:{id:string}[]}[]}>('get_agent_projection',{projectId});
  259 |   expect(prior.tasks).toHaveLength(1);
  260 |   await restored.getByRole('button',{name:'根据批注继续修改'}).click();
  261 |   const ai = page.getByRole('dialog',{name:'Focal AI',exact:true});
  262 |   await expect(ai.getByRole('button',{name:'取消方案引用'})).toBeVisible();
  263 |   expect((await rpc<typeof prior>('get_agent_projection',{projectId})).tasks).toHaveLength(1);
  264 |   await ai.getByRole('button',{name:'发送',exact:true}).click();
  265 |   await expect(ai.getByText('新的修订方案已准备好，原方案保留',{exact:true})).toBeVisible({timeout:30000});
  266 |   const result=await rpc<typeof prior>('get_agent_projection',{projectId});
  267 |   expect(result.tasks).toHaveLength(2); const child = result.tasks[0].boardDrafts[0].id; expect(child).not.toBe(draft);
  268 |   const modified=await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:child});
  269 |   expect(modified.nodes.some(n => n.contentJson?.includes('请用空镜作为中段过渡'))).toBe(true);
  270 |   expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  271 |   expect(await rpc('get_project',{projectId})).toEqual(official);
  272 |   const comments=await rpc<{messages:{resolved:boolean}[]}>('canvas_editorial_discussion',{projectId,draftId:draft,revision:parent.board.revision,offset:0,cursor:null});
  273 |   expect(comments.messages).toHaveLength(2); expect(comments.messages.every(m=>!m.resolved)).toBe(true);
  274 |   await page.screenshot({path:resolve(directory,'review-to-independent-revision.png')});
  275 |   // 修订依据取委托时的父版快照；晚到的意见仍留在父版，不重锚到新方案。
  276 |   await rpc('canvas_execute',{request:{schemaVersion:3,commandId:randomUUID(),idempotencyKey:randomUUID(),projectId,boardId:draft,baseRevision:parent.board.revision,operations:[{opId:randomUUID(),action:{type:'AddEditorialComment',data:{draftId:draft,draftRevision:parent.board.revision,changeId:null,threadId:null,body:'晚到意见，留给下一次修订'}}}]}});
  277 |   const context = await rpc<{discussion:{totalMessages:number;messages:{body:string}[]}}>('canvas_editorial_revision_context',{projectId,draftId:child,offset:0});
  278 |   expect(context.discussion.totalMessages).toBe(2);expect(context.discussion.messages.some(m=>m.body.includes('晚到'))).toBe(false);
  279 |   await page.goto('about:blank');await rpc('test_process_restart');
  280 |   await page.goto('/#/project/'+projectId);await page.getByRole('button',{name:'叙事画布',exact:true}).click();
  281 |   await page.getByRole('combobox',{name:'选择画布'}).selectOption(child);await page.getByRole('button',{name:'审查改动',exact:true}).click();
  282 |   const revisionContext=page.locator('.editorial-revision-context');await revisionContext.locator('summary').click();
  283 |   await expect(revisionContext.getByText('请用空镜作为中段过渡',{exact:true})).toBeVisible();
  284 |   await expect(revisionContext.getByText('晚到意见，留给下一次修订',{exact:true})).toHaveCount(0);
  285 |   await expect(revisionContext.getByRole('textbox')).toHaveCount(0);
  286 |   await revisionContext.scrollIntoViewIfNeeded();
  287 |   const contextDirectory=evidenceDirectory('cross-round-context');await mkdir(contextDirectory,{recursive:true});
  288 |   await page.screenshot({path:resolve(contextDirectory,'frozen-parent-context.png')});
  289 |   expect(await rpc('canvas_snapshot',{projectId,boardId:draft})).toEqual(parent);
  290 |   expect(await rpc('get_project',{projectId})).toEqual(official);
  291 |   expect((await rpc<Snapshot>('canvas_snapshot',{projectId,boardId:source})).board.revision).toBeGreaterThan(0);
  292 |   expect(errors).toEqual([]); expect(outside).toEqual([]);
  293 | });
  294 | 
```