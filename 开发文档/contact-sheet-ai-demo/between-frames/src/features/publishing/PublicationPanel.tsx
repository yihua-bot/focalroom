// G7 Publication Panel：桌面端出版物管理面板。
// 创建出版物、从序列生成初稿、查看页面列表。
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/shared/Button';

import type { Photo } from '../../domain/models';
import type { PublicationInfo, PublicationPage, PublicationRepository } from '../../domain/publication';
import { PublicationStudio } from './PublicationStudio';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';
import { useUIStore } from '../../stores/uiStore';
import { Modal } from '../../components/shared/Modal';

type Props={
  repository?: PublicationRepository;
  projectId: string;
  sequenceCount: number;
  photos?:Photo[];sequencePhotoIds?:string[];selectedPhotoIds?:string[];
  readOnly?: boolean;
};
export function PublicationPanel(props:Props){return <PublicationPanelBody key={props.projectId} {...props}/>;}
function PublicationPanelBody({repository,projectId,sequenceCount,photos=[],sequencePhotoIds=[],selectedPhotoIds=[],readOnly=false}:Props){
  const [publications, setPublications] = useState<PublicationInfo[]>([]);
  const [creating, setCreating] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [title, setTitle] = useState('');
  const [perPage, setPerPage] = useState<Record<string,number>>({});
  const [createdId,setCreatedId]=useState<string>();
  const focusedCreation=useRef<string|undefined>(undefined);
  const [notice, setNotice] = useState('');
  const [pages, setPages] = useState<Record<string, PublicationPage[]>>({});
  const [loadingPages, setLoadingPages] = useState<string | null>(null);
  const [editing,setEditing]=useState<PublicationInfo>();
  const [confirming,setConfirming]=useState(false);
  const [regenerate,setRegenerate]=useState<string>();
  const writeLock=useRef(false),mounted=useRef(true);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;};},[]);
  const pending=repository?.pendingCommands?.(projectId)??[];

  const load = async () => {
    if (!repository) return;
    const list = await repository.list(projectId);
    setPublications(list);
  };

  useEffect(() => {
    let active = true;
    if (repository) {
      repository.list(projectId).then(list => {
        if (active) setPublications(list);
      }).catch(error => { if (active) setNotice(userFacingError(error, zh.publicationLoadFailed)); });
    }
    return () => { active = false; };
  }, [repository, projectId]);

  const create = async () => {
    if (!repository || readOnly || creating || writeLock.current || !title.trim()) return;
    writeLock.current=true;
    setCreating(true);
    setNotice('');
    try {
      const created=await repository.create(projectId, title.trim());
      setTitle('');
      await load();
      setCreatedId(created.id);setNotice(zh.publicationCreatedGuide);
    } catch (error) {
      setNotice(userFacingError(error,zh.publicationCreateFailed));
    } finally { writeLock.current=false;setCreating(false); }
  };

  const generate = async (pubId: string, confirmed = false) => {
    if (!repository || readOnly || generating || writeLock.current) return;
    writeLock.current=true;
    setGenerating(true);
    setNotice('');
    try {
      if (!confirmed && repository.document && (await repository.document(projectId, pubId)).layout.pages.length > 0) {
        setRegenerate(pubId);
        return;
      }
      const count = await repository.generate(projectId, pubId, perPage[pubId]??1, confirmed);
      setRegenerate(undefined);
      setNotice(zh.publicationGenerated.replace('{count}', String(count)));
      setPages(current => ({ ...current, [pubId]: [] }));
      const list = await repository.list(projectId);
      if (!mounted.current) return;
      setPublications(list);
      if (repository.document && repository.versions && repository.version) setEditing(list.find(pub => pub.id === pubId));
      else setPages(current => ({ ...current, [pubId]: [] }));
    } catch (e) {
      setNotice(userFacingError(e, zh.publicationGenerateFailed));
    } finally { writeLock.current=false;setGenerating(false); }
  };

  const showPages = async (publicationId: string) => {
    if (!repository || loadingPages) return;
    setLoadingPages(publicationId);
    setNotice('');
    try {
      const loaded = await repository.pages(projectId, publicationId);
      setPages(current => ({ ...current, [publicationId]: loaded }));
      if (!loaded.length) setNotice(zh.publicationNoPages);
    } catch (error) {
      setNotice(userFacingError(error, zh.publicationLoadFailed));
    } finally { setLoadingPages(null); }
  };

  if (!repository) {
    return <div className="publication-empty"><p>{zh.publicationDesktop}</p></div>;
  }

  return <section className="publication-panel">
    <h2>{zh.publicationTitle}</h2>
    <p className="muted">{zh.publicationPurpose}</p>
    <ol className="publication-workflow"><li>{zh.publicationStepSequence}</li><li>{zh.publicationStepLayout}</li><li>{zh.publicationStepVersion}</li></ol>
    {sequenceCount === 0 && <div className="info-card"><p>{zh.publicationNeedSequence}</p><Button onClick={() => useUIStore.getState().setView('sequence')}>{zh.publicationGoSequence}</Button></div>}
    {pending.map(request=><Button key={request.publicationId} disabled={confirming||readOnly} onClick={()=>{
      if(!repository?.retryPending||writeLock.current)return;writeLock.current=true;setConfirming(true);const project=projectId;
      void repository.retryPending(project,request.publicationId).then(async()=>{if(!mounted.current)return;await load();if(mounted.current)setNotice(zh.publicationConfirmed);}).catch(error=>{if(mounted.current)setNotice(userFacingError(error,zh.publicationPending));}).finally(()=>{writeLock.current=false;if(mounted.current)setConfirming(false);});
    }}>{zh.publicationConfirmPrevious}</Button>)}

    <div className="publication-create">
      <label htmlFor="publication-title">{zh.publicationNewBook}</label>
      <input
        id="publication-title"
        type="text"
        value={title}
        placeholder={zh.publicationTitlePlaceholder}
        maxLength={200}
        disabled={creating || readOnly}
        aria-label={zh.publicationTitleLabel}
        onChange={e => setTitle(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter') void create(); }}
      />
      <Button size="small" disabled={creating || readOnly || !title.trim()} onClick={() => void create()}>
        {creating ? zh.publicationCreating : zh.publicationCreate}
      </Button>
    </div>

    {publications.map(pub => (
      <div key={pub.id} className={`publication-card${createdId===pub.id?' is-new':''}`} tabIndex={-1} ref={element=>{if(element&&createdId===pub.id&&focusedCreation.current!==pub.id){focusedCreation.current=pub.id;element.focus({preventScroll:true});element.scrollIntoView?.({block:'nearest',behavior:'smooth'});}}}>
        <div className="publication-card-head">
          <strong>{pub.title}</strong>
          <span className={`publication-state ${pub.state}`}>{zh[`publicationState_${pub.state}`]}</span>
        </div>
        <div className="publication-card-meta">
          {zh[`publicationType_${pub.pubType}`]} · {new Date(pub.createdAt).toLocaleDateString()}
        </div>
        <div className="publication-actions">
          <select aria-label={`${pub.title} · ${zh.publicationPhotosPerPage}`} value={perPage[pub.id]??1} disabled={readOnly || generating} onChange={e => setPerPage(previous=>({...previous,[pub.id]:Number(e.target.value)}))}>
            {[1,2,4].map(count => <option key={count} value={count}>{zh.publicationPerPage.replace('{count}', String(count))}</option>)}
          </select>
          <Button size="small" disabled={readOnly || generating || sequenceCount === 0 || pub.state !== 'draft'} onClick={() => void generate(pub.id)}>
            {generating ? zh.publicationGenerating : zh.publicationGenerateCount.replace('{count}', String(sequenceCount))}
          </Button>
          {!(repository.document&&repository.versions&&repository.version)&&<Button size="small" disabled={!!loadingPages || generating} onClick={() => void showPages(pub.id)}>
            {loadingPages === pub.id ? zh.publicationLoading : zh.publicationViewPages}
          </Button>}
          {repository.document&&repository.versions&&repository.version&&<Button size="small" variant="primary" disabled={creating||generating||confirming} onClick={()=>setEditing(pub)}>{zh.publicationStudio}</Button>}
        </div>
        {!!pages[pub.id]?.length && <div className="publication-pages" aria-label={`${pub.title}页面初稿`}>
          {pages[pub.id].map(page => <figure key={page.id} className="publication-page-preview">
            <div className="publication-paper" style={{ aspectRatio: `${page.widthMm} / ${page.heightMm}` }}>
              {page.frames.map(frame => <div key={frame.id} className="publication-frame" style={{
                left: `${frame.xMm / page.widthMm * 100}%`, top: `${frame.yMm / page.heightMm * 100}%`,
                width: `${frame.widthMm / page.widthMm * 100}%`, height: `${frame.heightMm / page.heightMm * 100}%`,
              }}>{frame.thumbnailSrc ? <img src={frame.thumbnailSrc} alt="" style={{ objectFit: frame.fitMode === 'fit' ? 'contain' : frame.fitMode === 'fill' ? 'cover' : 'fill' }} />
                : <span>{zh.publicationPreviewUnavailable}</span>}</div>)}
            </div>
            <figcaption>{page.pageNumber === 0 ? zh.publicationCover : zh.publicationPageNumber.replace('{number}', String(page.pageNumber))}</figcaption>
          </figure>)}
        </div>}
      </div>
    ))}

    {publications.length === 0 && (
      <p className="publication-hint">{zh.publicationStartHint}</p>
    )}

    {notice && <p role="status" className="publication-notice">{notice}</p>}
    <p className="micro">{zh.publicationOutputScope}</p>
    {regenerate&&<Modal title={zh.publicationRegenerate} description={zh.publicationRegenerateHint} onClose={()=>{if(!generating)setRegenerate(undefined);}}><div className="dialog-actions"><Button disabled={generating} onClick={()=>setRegenerate(undefined)}>{zh.publicationBack}</Button><Button variant="primary" disabled={generating} onClick={()=>void generate(regenerate,true)}>{zh.publicationRegenerateConfirm}</Button></div></Modal>}
    {editing&&<PublicationStudio key={`${projectId}:${editing.id}`} repository={repository} projectId={projectId} publication={editing} photos={photos} sequencePhotoIds={sequencePhotoIds} selectedPhotoIds={selectedPhotoIds} readOnly={readOnly} onClose={()=>setEditing(undefined)} onChanged={async()=>{setPages({});await load();}}/>}
  </section>;
}
