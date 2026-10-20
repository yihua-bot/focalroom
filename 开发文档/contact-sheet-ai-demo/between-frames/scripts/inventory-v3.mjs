// 从实际文件和已运行的隔离数据库生成 G0 盘点，不执行迁移或生产请求。
import { readFile, readdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, relative } from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const root = fileURLToPath(new URL('..', import.meta.url));
const [snapshotFile, cloudFile, output] = process.argv.slice(2);
if (!snapshotFile || !cloudFile || !output) throw new Error('用法：node scripts/inventory-v3.mjs <SQLite证据JSON> <PG证据JSON> <输出JSON>');
const hash = data => createHash('sha256').update(data).digest('hex');
const source = async path => ({ path, sha256: hash(await readFile(resolve(root, path))) });
const localFiles = (await readdir(resolve(root, 'src-tauri/migrations'))).filter(name => /^\d{4}_.+\.sql$/.test(name)).sort();
const registry = await readFile(resolve(root, 'apps/api/src/database.ts'), 'utf8');
const registered = [...registry.matchAll(/"(\d{4}_[^"\n]+\.sql)"/g)].map(match => match[1]);
const allCloud = (await readdir(resolve(root, 'apps/api/migrations'))).filter(name => name.endsWith('.sql')).sort();
const modules = [
 ['ai-drawer','src/features/ai-editor/AISelectionDrawer.tsx','workspace conversation drawer; nested Escape guarded'],
 ['desktop-ui','src/app/AppController.tsx','working selection workspace; no free-coordinate Board'],
 ['local-authority','src-tauri/src/command_handler.rs','validated formal local commands, revision, idempotency, audit'],
 ['sqlite','src-tauri/src/db.rs','backup-before-upgrade and SQLx migrations'],
 ['image-preview','src-tauri/src/image_pipeline.rs','320/1400px RGB8 JPEG; not high-bit-depth professional rendering'],
 ['original-inspection','src-tauri/src/photo_inspection.rs','bounded original/embedded/derived source; not 100MP native tiles'],
 ['agent','src-tauri/src/agent/runtime.rs','local task/tool/checkpoint runtime; model writes drafts'],
 ['agent-policy','src-tauri/src/agent/policy.rs','resource scope, budgets and tool boundaries'],
 ['memory','src-tauri/src/agent/memory.rs','evidence/hypotheses/correction; not professionally validated multimodal preference learning'],
 ['sequence','src/components/sequence/SequenceBoard.tsx','linear sequence; distinct from future Narrative Canvas'],
 ['shared-api','apps/api/src/authority.ts','PostgreSQL shared formal state and approved commands'],
 ['cloud-worker','apps/api/src/agent-tasks.ts','comment summary/disagreement; not full Canvas Agent'],
 ['web-review','apps/web/src/shared/ProjectReview.tsx','shared photo review; no v3 Board UI'],
 ['export','src-tauri/src/export.rs','compatibility CSV/JSON/custom sidecar/HTML proof; not PDF/X'],
 ['contracts','packages/contracts/src/index.ts','current v2 semantic contracts; v3 blueprints not runtime'],
];
const localSnapshot = JSON.parse(await readFile(snapshotFile, 'utf8'));
const cloud = JSON.parse(await readFile(cloudFile, 'utf8'));
const git = args => execFileSync('git', args, {cwd:root,encoding:'utf8'}).trim().split('\n').filter(Boolean);
const changed = [...new Set([...git(['diff','--name-only','--relative','--','src','src-tauri','apps','scripts','README.md']),...git(['ls-files','--others','--exclude-standard','--','src','src-tauri','apps','scripts'])])];
const result = {
 schemaVersion: 1, product: 'FocalRoom', target: 'v3.0', gate: 'G0', observedAt: new Date().toISOString(),
 sourceHead: execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),
 changedSources: await Promise.all(changed.filter(path => /\.(tsx?|rs|mjs|md|json)$/.test(path)).map(source)),
 branch: execFileSync('git',['branch','--show-current'],{cwd:root,encoding:'utf8'}).trim(),
 clientVersion: JSON.parse(await readFile(resolve(root,'src-tauri/tauri.conf.json'),'utf8')).version,
 ciWorkflow: await source('../../../.github/workflows/betweenframes-v2.yml'),
 status: 'LOCAL_VERIFICATION_IN_PROGRESS_REMOTE_CI_PENDING',
 scope: { userDataOpened:false, productionChanged:false, realModelCalls:0, historicalMigrationsChanged:false },
 modules: await Promise.all(modules.map(async ([id,path,boundary]) => ({id,...await source(relative(root,resolve(root,path))),boundary}))),
 database: {
  sqlite: { migrationFiles: await Promise.all(localFiles.map(name=>source('src-tauri/migrations/'+name))), ...localSnapshot },
  postgresql: { ...cloud, registeredFiles:await Promise.all(registered.map(name=>source('apps/api/migrations/'+name))), unregisteredDesignFiles:allCloud.filter(name=>!registered.includes(name)) }
 },
 tests: [],
 remaining: ['Remote CI for the corrected commit','G1/G2 API/data/color design review','No v3 Asset/Canvas/Color/Bridge/Publishing implementation in this gate','Human/device/supplier/production acceptance remains separate']
};
await writeFile(output,JSON.stringify(result,null,2)+'\n');
