import { act,fireEvent,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,Project } from '../../domain/models';
import type { DraftEvidence, SelectionDraft } from '../../domain/agent';
import { setCopyLocale, zh } from '../../app/copy';
import { collectDraftDoubts, DraftReview } from './DraftReview';
import type { PhotoInspection } from '../../domain/agent';
import type { PhotoJudgment } from '../../domain/photoJudgment';
const project:Project={id:'p',title:'作品',brief:'',revision:1,photos:[{id:'a',name:'a.jpg',src:'/a.jpg',index:1,status:'undecided',note:'',rating:0},{id:'b',name:'b.jpg',src:'/b.jpg',index:2,status:'undecided',note:'',rating:0}],sequence:[],events:[]};
const draft:SelectionDraft={id:'draft',projectId:'p',taskId:'task',title:'选片方案',baseRevision:1,revision:1,state:'pending',stale:false,summary:'保留人物',items:[{photo_id:'a',proposed_action:'keep',position:0,rationale:'开篇'},{photo_id:'b',proposed_action:'keep',position:1,rationale:'过渡'}],createdAt:'now'};
function setup(value=draft){const repository={agent:{},applyDraft:vi.fn().mockResolvedValue(project),discardDraft:vi.fn().mockResolvedValue(project),rebaseDraft:vi.fn().mockResolvedValue(project),updateDraft:vi.fn().mockResolvedValue(project)};const onUpdated=vi.fn(),onClose=vi.fn();render(<DraftReview draft={value} project={project} repository={repository as unknown as DesktopProjectRepository} onClose={onClose} onUpdated={onUpdated}/>);return {repository,onUpdated,onClose};}
afterEach(()=>setCopyLocale('zh'));
it('只发送明确勾选的照片，等待回执期间阻止重复采纳',async()=>{
  const {repository,onUpdated}=setup();const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'清除选择'}));await user.click(screen.getByRole('checkbox',{name:'采纳 a.jpg'}));
  let finish:(value:Project)=>void=()=>{};repository.applyDraft.mockImplementationOnce(()=>new Promise<Project>(resolve=>{finish=resolve;}));
  const button=screen.getByRole('button',{name:'采纳所选 1 张'});fireEvent.click(button);fireEvent.click(button);
  expect(repository.applyDraft).toHaveBeenCalledTimes(1);expect(repository.applyDraft).toHaveBeenCalledWith('p',draft,['a'],'partial');expect(onUpdated).not.toHaveBeenCalled();await act(async()=>finish(project));expect(onUpdated).toHaveBeenCalledOnce();
});
it('替换整个作品序列必须再次明确确认',async()=>{
  const {repository}=setup();const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'替换作品序列'}));expect(repository.applyDraft).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'确认替换'}));expect(repository.applyDraft).toHaveBeenCalledWith('p',draft,['a','b'],'full');
});
it('已处理的方案不把全部建议显示为已勾选采纳，也不显示写入入口',()=>{
  setup({...draft,state:'applied'});expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'全选方案照片'})).not.toBeInTheDocument();expect(screen.getByText('这份方案已结束审核。采纳记录可在工作台撤销。')).toBeVisible();
});
it.each([['en','Replace sequence'],['ja','作品シーケンスを置き換え'],['ko','작품 시퀀스 교체']])('选片审核支持%s，不残留中文操作词', (locale,label)=>{
  setCopyLocale(locale);setup();expect(screen.getByRole('button',{name:label})).toBeVisible();expect(screen.queryByText('清除选择')).not.toBeInTheDocument();
});
const evidence:DraftEvidence={draftId:'draft',count:5,observed:2,unobserved:3,pageTier:1,detailTier:1,otherVersion:0,otherTool:0,crossContractPairs:1,omitted:2,scanned:2,draftRevision:1,perPhoto:[{photoId:'a',contract:'current_detail_tier',used:true},{photoId:'b',contract:null,used:false,pixelVersion:'unreadable'}]};
const withEvidence=(value?:DraftEvidence,agent:Record<string,unknown>={})=>{const repository={agent};render(<DraftReview draft={draft} project={project} evidence={value} repository={repository as unknown as DesktopProjectRepository} onClose={vi.fn()} onUpdated={vi.fn()}/>);return repository as unknown as {agent:{recheckEvidence?:ReturnType<typeof vi.fn>}}};
it('方案里明说有画面观察的是几张、按哪档问法挑的，跨档比较单独点出来',()=>{
  withEvidence(evidence);
  expect(screen.getByText('这份方案的画面依据：2 张有可核对的画面观察，3 张没有。')).toBeVisible();
  expect(screen.getByText(/当前页面档 1 张 · 大尺寸细节档 1 张/)).toBeVisible();
  expect(screen.getByText('有 1 处相邻两张不是同一档问法问出来的，不能当成同一把尺子比。')).toBeVisible();
  expect(screen.getByText('另有 2 张的画面记录因为原图换过或本地预览读不出来，没有采用。')).toBeVisible();
});
it('一张观察都没有时说明只靠评分状态顺序，零计数不写成排除清单',()=>{
  withEvidence({...evidence,observed:0,unobserved:5,pageTier:0,detailTier:0,crossContractPairs:0,omitted:0,scanned:0,perPhoto:[]});
  expect(screen.getByText('这份方案的照片都还没有可用的画面观察，下面的判断只靠评分、状态与顺序。')).toBeVisible();
  expect(screen.queryByText(/没有采用/)).not.toBeInTheDocument();
  expect(screen.queryByText(/不能当成同一把尺子比/)).not.toBeInTheDocument();
});
it('没有核验回执时不编一段画面依据',()=>{
  withEvidence(undefined);
  expect(screen.queryByText(/画面依据/)).not.toBeInTheDocument();
  expect(screen.queryByText(/按问法分档/)).not.toBeInTheDocument();
});
it('逐张说清这张按哪档看的、那张为什么没用上，记档齐了就不加截断说明',()=>{
  withEvidence(evidence);
  expect(screen.getByText('这张按大尺寸细节档看的画面，比页面档看得更细')).toBeVisible();
  expect(screen.getByText('这张的画面记录没用上：本地预览读不出来')).toBeVisible();
  expect(screen.queryByText(/只算进总数/)).not.toBeInTheDocument();
});
it('后端没记这张就什么都不说，不拿"没有记录"当成"没有观察"；有观察的多于记档时才补一句',()=>{
  withEvidence({...evidence,observed:4,scanned:1,perPhoto:[{photoId:'a',contract:'current_page_tier',used:true}]});
  expect(screen.getByText('这张按当前页面档看的画面')).toBeVisible();
  // b.jpg 没有任何逐张记录：既不能说它没观察，也不能说它按哪档看的——所以整份方案只有一句逐张口径。
  expect(document.querySelectorAll('.draft-tier')).toHaveLength(1);
  expect(screen.getByText('逐张口径只点到上面列出的那几张，其余有画面观察的照片只算进总数。')).toBeVisible();
});
it('逐张口径铺到整份方案，排在第十三位之后的照片也有自己的说法',()=>{
  const ids=Array.from({length:14},(_,index)=>`p${index}`);
  const many:Project={...project,photos:ids.map((id,index)=>({id,name:`${id}.jpg`,src:`/${id}.jpg`,index:index+1,status:'undecided',note:'',rating:0}))};
  const long:SelectionDraft={...draft,items:ids.map((id,index)=>({photo_id:id,proposed_action:'keep',position:index,rationale:'序列顺序'}))};
  // 前十二张按页面档问、后面两张按细节档问：后面这两张正是过去只会算进总数的那几张。
  const perPhoto=ids.map((photoId,index)=>({photoId,contract:index<12?'current_page_tier' as const:'current_detail_tier' as const,used:true}));
  render(<DraftReview draft={long} project={many} evidence={{...evidence,count:14,observed:14,unobserved:0,pageTier:12,detailTier:2,crossContractPairs:0,omitted:0,scanned:14,perPhoto}} repository={{agent:{}} as unknown as DesktopProjectRepository} onClose={vi.fn()} onUpdated={vi.fn()}/>);
  expect(document.querySelectorAll('.draft-tier')).toHaveLength(14);
  expect(screen.getAllByText('这张按大尺寸细节档看的画面，比页面档看得更细')).toHaveLength(2);
  expect(screen.queryByText(/只算进总数/)).not.toBeInTheDocument();
});
it('口径说明对着方案的哪一版算的，版本没变也讲明白',()=>{
  withEvidence(evidence);
  expect(screen.getByText('上面的口径是对着方案第 1 版算的，就是现在这一版。')).toBeVisible();
});
it('方案改过之后不拿旧口径冒充新口径：明说中间改过、口径没重算',()=>{
  withEvidence({...evidence,draftRevision:4});
  expect(screen.getByText('上面的口径是对着方案第 4 版算的，现在已是第 1 版：中间改过，口径没重算。')).toBeVisible();
  expect(screen.queryByText(/就是现在这一版/)).not.toBeInTheDocument();
});
it('老回执没记版本号就不编一个，宁可不提这一句',()=>{
  withEvidence({...evidence,draftRevision:null});
  expect(screen.queryByText(/上面的口径是对着方案第/)).not.toBeInTheDocument();
  expect(screen.getByText('这份方案的画面依据：2 张有可核对的画面观察，3 张没有。')).toBeVisible();
});
it('口径可能对不上时给一个本地重算的动作，说清不重问模型也不上传照片',async()=>{
  const repository=withEvidence({...evidence,draftRevision:4}, {recheckEvidence:vi.fn().mockResolvedValue({draftId:'draft',draftRevision:1,count:5,observed:2,unobserved:3,omitted:2})});
  expect(screen.getByText(/不重新问模型，也不上传照片/)).toBeVisible();
  await userEvent.setup().click(screen.getByRole('button',{name:'按现在这一版重算口径'}));
  expect(repository.agent.recheckEvidence).toHaveBeenCalledWith('p','draft');
  expect(await screen.findByText('已重算一次，口径按现在这一版重新记了。')).toBeVisible();
});
it('口径就是现在这一版、也没有排除照片时不摆重算按钮；仓库不支持重算时同样不摆',()=>{
  withEvidence({...evidence,omitted:0}, {recheckEvidence:vi.fn()});
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
  withEvidence(evidence);
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
});
it('重算失败只在对话框里说，不拿成功的话糊过去',async()=>{
  const repository=withEvidence(evidence, {recheckEvidence:vi.fn().mockRejectedValue(new Error('方案已不在库里，请刷新后重试'))});
  await userEvent.setup().click(screen.getByRole('button',{name:'按现在这一版重算口径'}));
  expect(repository.agent.recheckEvidence).toHaveBeenCalledOnce();
  expect(screen.getByRole('alert')).toHaveTextContent('方案已不在库里，请刷新后重试');
  expect(screen.queryByText('已重算一次，口径按现在这一版重新记了。')).not.toBeInTheDocument();
});
it('从没算过口径的方案（画布里生成的那份序列草案最常见）也给一个本地算一次的入口',async()=>{
  const repository=withEvidence(undefined, {recheckEvidence:vi.fn().mockResolvedValue({draftId:'draft',draftRevision:1,count:3,observed:3,unobserved:0,omitted:0})});
  expect(screen.getByText('这份方案还没有能核对的画面依据，下面的判断只靠评分、状态与顺序。')).toBeVisible();
  expect(screen.getByText(/不重新问模型，也不上传照片/)).toBeVisible();
  await userEvent.setup().click(screen.getByRole('button',{name:'在本地算一次画面依据'}));
  expect(repository.agent.recheckEvidence).toHaveBeenCalledWith('p','draft');
  expect(await screen.findByText('已在本地算过一次。这份的口径读得回来时，这里会换成算出来的数字。')).toBeVisible();
});
it('算过但核验单整条读不回来的那份：计数照说，逐张档位照实说没读回来',()=>{
  withEvidence({...evidence, basisSummary:true, perPhoto:[]});
  expect(screen.getByText('这份方案算过，只是那份核验单没能整条读回来，逐张的档位这里就不写了。')).toBeVisible();
});
it('重算完数字一位没变就照实说和刚才一样，不假装有了进展',async()=>{
  const repository=withEvidence({...evidence,draftRevision:1}, {recheckEvidence:vi.fn().mockResolvedValue({draftId:'draft',draftRevision:1,count:5,observed:2,unobserved:3,omitted:2})});
  await userEvent.setup().click(screen.getByRole('button',{name:'按现在这一版重算口径'}));
  expect(repository.agent.recheckEvidence).toHaveBeenCalledOnce();
  expect(await screen.findByText('算了一遍，结果和刚才一样：那 2 张的画面记录还是用不上。再算也是这个数——先修它们的本地预览，修好再算一次才可能变；原图本身换过、或者原图不在这台机器上的那几张，修也修不回来，只能重新问一次模型。')).toBeVisible();
  expect(screen.queryByText('已重算一次，口径按现在这一版重新记了。')).not.toBeInTheDocument();
});
it.each([['en','Recomputed, and the result is the same as before: the visual records for those 2 photos still cannot be used. Computing again gives the same numbers — repair their local previews and recompute once more before expecting a change; any photo whose original has itself been replaced, or whose original is not on this machine, cannot be brought back by repairing, and asking the model again is the only way.'],['ja','計算し直しましたが、結果は先ほどと同じです。その 2 枚の画面記録はやはり使えません。もう一度計算しても同じ数字です。まずローカルプレビューを直し、直してからもう一度計算して初めて変わる可能性があります。原本そのものが差し替わっていたり、この機器に原本がない枚は直しても戻らず、モデルにもう一度尋ねるしかありません。'],['ko','다시 계산했지만 결과는 아까와 같습니다. 그 2장의 화면 기록은 여전히 쓸 수 없습니다. 또 계산해도 같은 숫자입니다. 먼저 로컬 미리보기를 복구하고 다시 계산해야 바뀔 수 있습니다. 원본 자체가 바뀌었거나 이 기기에 원본이 없는 장은 복구해도 되돌릴 수 없어 모델에게 다시 물어보는 수밖에 없습니다.']])('空转那句连"修不回来"的例外一起支持%s，不残留中文',async(locale,text)=>{
  setCopyLocale(locale);
  withEvidence({...evidence,draftRevision:1}, {recheckEvidence:vi.fn().mockResolvedValue({draftId:'draft',draftRevision:1,count:5,observed:2,unobserved:3,omitted:2})});
  await userEvent.setup().click(screen.getByRole('button',{name:zh.draftEvidenceRecheck}));
  expect(await screen.findByText(text)).toBeVisible();
});
it('重算真把排除的那几张补回来了，就照旧说已按现在这一版重新记过',async()=>{
  withEvidence({...evidence,draftRevision:1}, {recheckEvidence:vi.fn().mockResolvedValue({draftId:'draft',draftRevision:1,count:5,observed:4,unobserved:1,omitted:0})});
  await userEvent.setup().click(screen.getByRole('button',{name:'按现在这一版重算口径'}));
  expect(await screen.findByText('已重算一次，口径按现在这一版重新记了。')).toBeVisible();
  expect(screen.queryByText(/结果和刚才一样/)).not.toBeInTheDocument();
});
it('比对用的是点击那一刻的数：等待期间投影已经刷新过，也不把真有进展说成空转',async()=>{
  let finish:(value:{draftId:string;draftRevision:number;count:number;observed:number;unobserved:number;omitted:number})=>void=()=>{};
  const repository={agent:{recheckEvidence:vi.fn().mockImplementation(()=>new Promise(resolve=>{finish=resolve;}))}};
  const props={draft,project,repository:repository as unknown as DesktopProjectRepository,onClose:vi.fn(),onUpdated:vi.fn()};
  const view=render(<DraftReview {...props} evidence={{...evidence,draftRevision:1}}/>);
  fireEvent.click(screen.getByRole('button',{name:'按现在这一版重算口径'}));
  view.rerender(<DraftReview {...props} evidence={{...evidence,draftRevision:1,observed:3,unobserved:2}}/>);
  await act(async()=>finish({draftId:'draft',draftRevision:1,count:5,observed:3,unobserved:2,omitted:2}));
  expect(await screen.findByText('已重算一次，口径按现在这一版重新记了。')).toBeVisible();
  expect(screen.queryByText(/结果和刚才一样/)).not.toBeInTheDocument();
});
const unrecorded:DraftEvidence={...evidence,draftRevision:1,perPhoto:[{photoId:'a',contract:'current_detail_tier',used:true},{photoId:'b',contract:null,used:false,pixelVersion:'unrecorded'},{photoId:'c',contract:null,used:false,pixelVersion:'unrecorded'}]};
it('排除的全是当初没记像素指纹、版本号也已对上时，不摆那个注定白点的按钮，照实说原因',()=>{
  withEvidence(unrecorded,{recheckEvidence:vi.fn()});
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
  expect(screen.getByText('这 2 张的画面记录当初没记下看的是哪份图像，本地补不出来，再算也还是用不上。要让它们有画面依据，只能重新问一次模型。')).toBeVisible();
});
it('排除里只要有一张是预览读不出来，修好就还有救，按钮照旧摆着',()=>{
  withEvidence({...unrecorded,perPhoto:[...unrecorded.perPhoto.slice(0,2),{photoId:'c',contract:null,used:false,pixelVersion:'unreadable'}]},{recheckEvidence:vi.fn()});
  expect(screen.getByRole('button',{name:'按现在这一版重算口径'})).toBeVisible();
  expect(screen.queryByText(/本地补不出来/)).not.toBeInTheDocument();
});
it('逐张原因没看全（超出上限被截）就不敢断言白点，按钮照旧',()=>{
  withEvidence({...unrecorded,perPhoto:unrecorded.perPhoto.slice(0,2)},{recheckEvidence:vi.fn()});
  expect(screen.getByRole('button',{name:'按现在这一版重算口径'})).toBeVisible();
  expect(screen.queryByText(/本地补不出来/)).not.toBeInTheDocument();
});
it('降级那份没有逐张原因，同样不敢断言白点',()=>{
  withEvidence({...unrecorded,basisSummary:true,perPhoto:[]},{recheckEvidence:vi.fn()});
  expect(screen.getByRole('button',{name:'按现在这一版重算口径'})).toBeVisible();
  expect(screen.queryByText(/本地补不出来/)).not.toBeInTheDocument();
});
it('版本号还没对上时重算仍有正事要做，即使排除的全是没记指纹也照旧摆按钮',()=>{
  withEvidence({...unrecorded,draftRevision:4},{recheckEvidence:vi.fn()});
  expect(screen.getByRole('button',{name:'按现在这一版重算口径'})).toBeVisible();
  expect(screen.queryByText(/本地补不出来/)).not.toBeInTheDocument();
});
it.each([['en','The visual records for these 2 photos never recorded which image was looked at. That cannot be filled in locally, so recomputing still leaves them out; giving them visual evidence means asking the model again.'],['ja','これら 2 枚の画面記録は、どの画像を見たかが当初記録されておらず、この機器では補えません。計算し直してもやはり使えません。画面根拠を持たせるには、モデルにもう一度尋ねるしかありません。'],['ko','이 2장의 화면 기록은 처음에 어떤 이미지를 봤는지 기록되지 않아 이 기기에서는 채울 수 없습니다. 다시 계산해도 여전히 쓸 수 없습니다. 화면 근거를 갖게 하려면 모델에게 다시 물어보는 수밖에 없습니다.']])('白点那一格的说明支持%s，不残留中文', (locale,text)=>{
  setCopyLocale(locale);withEvidence(unrecorded,{recheckEvidence:vi.fn()});
  expect(screen.getByText(text)).toBeVisible();
  expect(screen.queryByText(/本地补不出来/)).not.toBeInTheDocument();
});
const missing:DraftEvidence={...evidence,draftRevision:1,perPhoto:[{photoId:'a',contract:'current_detail_tier',used:true},{photoId:'b',contract:null,used:false,pixelVersion:'unreadable',originalStatus:'missing'},{photoId:'c',contract:null,used:false,pixelVersion:'mismatch',originalStatus:'missing'}]};
it('排除的全是原图不在本机时同样不摆那个白点按钮，并且不再说成只是没记指纹',()=>{
  withEvidence(missing,{recheckEvidence:vi.fn()});
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
  expect(screen.getByText('这 2 张的画面依据本地补不回来：原图不在这台机器上、原图已经换过，或者当初没记下看的是哪份图像。再算也还是用不上，要让它们有画面依据只能重新问一次模型。')).toBeVisible();
  expect(screen.queryByText(/当初没记下看的是哪份图像，本地补不出来/)).not.toBeInTheDocument();
});
it('原图已经换过的那一类也判死，不摆白点按钮',()=>{
  withEvidence({...missing,omitted:1,perPhoto:[missing.perPhoto[0],{photoId:'b',contract:null,used:false,pixelVersion:'unreadable',originalStatus:'changed'}]},{recheckEvidence:vi.fn()});
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
  expect(screen.getByText(/原图不在这台机器上、原图已经换过/)).toBeVisible();
});
it.each(['matches','unchecked'] as const)('原图状态是%s时不敢断言白点：修好预览还有可能翻过来，按钮照旧',originalStatus=>{
  withEvidence({...missing,perPhoto:[missing.perPhoto[0],missing.perPhoto[1],{photoId:'c',contract:null,used:false,pixelVersion:'unreadable',originalStatus}]},{recheckEvidence:vi.fn()});
  expect(screen.getByRole('button',{name:'按现在这一版重算口径'})).toBeVisible();
  expect(screen.queryByText(/本地补不回来/)).not.toBeInTheDocument();
});
it('没记指纹和原图不在本机混在一起时说那一格通用的话，不冒充成全是没记指纹',()=>{
  withEvidence({...missing,perPhoto:[missing.perPhoto[0],missing.perPhoto[1],{photoId:'c',contract:null,used:false,pixelVersion:'unrecorded'}]},{recheckEvidence:vi.fn()});
  expect(screen.queryByRole('button',{name:'按现在这一版重算口径'})).not.toBeInTheDocument();
  expect(screen.getByText(/原图不在这台机器上、原图已经换过，或者当初没记下看的是哪份图像/)).toBeVisible();
  expect(screen.queryByText(/的画面记录当初没记下看的是哪份图像，本地补不出来/)).not.toBeInTheDocument();
});
it('逐张那一行点名真正的死因，不再只说预览读不出来',()=>{
  withEvidence(missing);
  expect(screen.getByText('这张的画面记录没用上：原图不在这台机器上，本地修不回来')).toBeVisible();
  expect(screen.queryByText('这张的画面记录没用上：本地预览读不出来')).not.toBeInTheDocument();
});
it('原图已经换过时逐张那一行也照实点名',()=>{
  withEvidence({...missing,perPhoto:[missing.perPhoto[0],{...missing.perPhoto[1],originalStatus:'changed'},missing.perPhoto[2]]});
  expect(screen.getByText('这张的画面记录没用上：原图已经换过，本地修不回来')).toBeVisible();
});
it.each(['matches','unchecked',null] as const)('原图状态是%s（含老回执没查过）时逐张照旧只说预览那一层',originalStatus=>{
  withEvidence({...missing,perPhoto:[missing.perPhoto[0],{photoId:'b',contract:null,used:false,pixelVersion:'unreadable',originalStatus},missing.perPhoto[2]]});
  expect(screen.getByText('这张的画面记录没用上：本地预览读不出来')).toBeVisible();
  expect(screen.queryByText(/原图不在这台机器上，本地修不回来/)).not.toBeInTheDocument();
});
it.each([['en','The visual evidence for these 2 photos cannot be filled in locally: the original is not on this machine, the original has been replaced, or which image was looked at was never recorded. Recomputing still leaves them out; giving them visual evidence means asking the model again.'],['ja','これら 2 枚の画面根拠はこの機器では補えません：原本がこの機器にない、原本が差し替わっている、またはどの画像を見たかが当初記録されていない、のいずれかです。計算し直してもやはり使えません。画面根拠を持たせるには、モデルにもう一度尋ねるしかありません。'],['ko','이 2장의 화면 근거는 이 기기에서 채울 수 없습니다: 원본이 이 기기에 없거나, 원본이 바뀌었거나, 처음에 어떤 이미지를 봤는지 기록되지 않았습니다. 다시 계산해도 여전히 쓸 수 없습니다. 화면 근거를 갖게 하려면 모델에게 다시 물어보는 수밖에 없습니다.']])('原图那一格的说明支持%s，不残留中文',(locale,text)=>{
  setCopyLocale(locale);withEvidence(missing,{recheckEvidence:vi.fn()});
  expect(screen.getByText(text)).toBeVisible();
  expect(screen.queryByText(/本地补不回来/)).not.toBeInTheDocument();
});
it.each([['en','missing','This photo\'s visual record was not used: its original is not on this machine, so repairing locally cannot bring it back'],['ja','missing','この枚の画面記録は使われませんでした：原本がこの機器になく、ローカルでは直せません'],['ko','changed','이 장의 화면 기록은 쓰이지 않았습니다: 원본이 바뀌어 로컬에서는 복구할 수 없습니다']] as const)('逐张点名的死因也支持%s，不残留中文',(locale,originalStatus,text)=>{
  setCopyLocale(locale);withEvidence({...missing,perPhoto:[missing.perPhoto[0],{...missing.perPhoto[1],originalStatus},missing.perPhoto[2]]});
  expect(screen.getByText(text)).toBeVisible();
  expect(screen.queryByText(/原图不在这台机器上，本地修不回来/)).not.toBeInTheDocument();
  expect(screen.queryByText(/原图已经换过，本地修不回来/)).not.toBeInTheDocument();
});
it('整项目的点名不再挂在这份方案的弹窗里：那是项目自己的文件事实，入口在图库一侧',()=>{
  const repository={agent:{},auditPreviews:vi.fn(),repairAllPreviews:vi.fn()};
  render(<DraftReview draft={draft} project={project} repository={repository as unknown as DesktopProjectRepository} onClose={vi.fn()} onUpdated={vi.fn()}/>);
  expect(screen.queryByRole('button',{name:'检查预览'})).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:'重生成坏预览'})).not.toBeInTheDocument();
  expect(repository.auditPreviews).not.toHaveBeenCalled();
});
const judged=(photoId:string,uncertainty:string,observationUncertainty:string):PhotoJudgment=>({id:'judgment-'+photoId,photoId,observationCallId:'call',judgment:'主体可能不够突出',reasoning:'明亮背景竞争注意力',uncertainty,observationUncertainty,facts:[],methods:[],imageStatus:'verified',inference:true});
const seen=(photoId:string,uncertainty:string):PhotoInspection=>({photo_id:photoId,visible_facts:['画面左侧有深色主体'],interpretation:'可能形成停顿',uncertainty});
it('判断层自己说了两层不准就以判断为准，观察记录不再重复覆盖同一格',()=>{
  expect(collectDraftDoubts({photoJudgments:[judged('a','还得看拍摄意图','表情看不清')],inspectedPhotos:[seen('a','预览远处细节看不清')]}))
    .toEqual([{photoId:'a',judgment:'还得看拍摄意图',observation:'表情看不清'}]);
});
it('判断层没提画面那层就退回观察记录，只有观察的照片也只说观察这一层',()=>{
  expect(collectDraftDoubts({photoJudgments:[judged('a','还得看拍摄意图','  ')],inspectedPhotos:[seen('a','预览远处细节看不清'),seen('b','裁切未定')]}))
    .toEqual([{photoId:'a',judgment:'还得看拍摄意图',observation:'预览远处细节看不清'},{photoId:'b',observation:'裁切未定'}]);
});
it('空话、没点名的记录不算一条不准：回执里没写就一行都不编',()=>{
  expect(collectDraftDoubts({photoJudgments:[judged('a','   ','')],inspectedPhotos:[seen('b',''),seen('','不该出现')]})).toEqual([]);
  expect(collectDraftDoubts(undefined)).toEqual([]);
});
it('草案里逐张写出 AI 自己留下的不准，没留不准的那张不编',()=>{
  render(<DraftReview draft={draft} project={project} repository={{agent:{}} as unknown as DesktopProjectRepository} doubts={[{photoId:'a',judgment:'还得看拍摄意图',observation:'预览远处细节看不清'}]} onClose={vi.fn()} onUpdated={vi.fn()}/>);
  expect(screen.getByText('这张的画面观察本身也说不准：预览远处细节看不清')).toBeVisible();
  expect(screen.getByText('这条取舍 AI 自己也留了不准：还得看拍摄意图')).toBeVisible();
  expect(screen.getAllByText(/^这条取舍 AI 自己也留了不准/)).toHaveLength(1);
  expect(screen.getAllByText(/^这张的画面观察本身也说不准/)).toHaveLength(1);
});
it.each([['en','The visual read of this photo is itself uncertain: 预览远处细节看不清'],['ja','この写真の画面観察自体も不確かです：预览远处细节看不清'],['ko','이 사진의 화면 관찰 자체도 불확실합니다: 预览远处细节看不清']])('这张说不准跟着语言走（%s）',(locale,text)=>{
  setCopyLocale(locale);
  render(<DraftReview draft={draft} project={project} repository={{agent:{}} as unknown as DesktopProjectRepository} doubts={[{photoId:'a',observation:'预览远处细节看不清'}]} onClose={vi.fn()} onUpdated={vi.fn()}/>);
  expect(screen.getByText(text)).toBeVisible();
  expect(screen.queryByText(/这张的画面观察本身也说不准/)).not.toBeInTheDocument();
});
