import type { PublicationDocument } from '../../domain/publication';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { zh } from '../../app/copy';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { PublicationPaper } from './PublicationPaper';
import type { PublicationGroupRemoval } from './publicationRemoval';

export function PublicationGroupRemovalDialog({plan,document,busy,pending,notice,onClose,onConfirm}:{plan:PublicationGroupRemoval;document:PublicationDocument;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onConfirm:()=>void}){
  const page=document.layout.pages.find(page=>page.id===plan.pageId)!,after=plan.preview.pages.find(page=>page.id===plan.pageId)!;
  const photos=plan.objects.filter(object=>object.kind==='frame').length,texts=plan.objects.length-photos;
  return <Modal title={zh.publicationRemoveGroupConfirm} description={zh.publicationRemoveGroupHint} onClose={()=>{if(!busy)onClose();}}>
    <p>{zh.publicationRemoveGroupSummary.replace('{name}',plan.name??zh.publicationGroup).replace('{photos}',String(photos)).replace('{texts}',String(texts))}</p>
    <div className="publication-removal-preview">
      <section><h4>{zh.publicationBeforeRemoval}</h4><PublicationPaper page={page} previews={document.previews} layers={publicationLayerViews(publicationLayers(document.layout))} label={zh.publicationBeforeRemoval}/></section>
      <section><h4>{zh.publicationAfterRemoval}</h4><PublicationPaper page={after} previews={document.previews} layers={publicationLayerViews(publicationLayers(plan.preview))} label={zh.publicationAfterRemoval}/></section>
    </div>
    <div className="dialog-actions"><Button disabled={busy||pending} onClick={onConfirm}>{zh.publicationRemoveGroupConfirm}</Button><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button></div>
    {notice&&<p role="status">{notice}</p>}
  </Modal>;
}
