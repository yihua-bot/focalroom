import type { StructuralConflict, StructuralResolution } from '../../domain/canvas';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function EditorialStructuralConflicts({ conflicts, disabled, canKeep, onChoose, onKeep }: {
  conflicts: StructuralConflict[]; disabled: boolean; canKeep: boolean;
  onChoose: (resolution: StructuralResolution) => void; onKeep: (changeId: string) => void;
}) {
  if (!conflicts.length) return null;
  return <section className="editorial-conflicts" aria-label={zh.editorialStructuralConflicts}>
    <h4>{zh.editorialStructuralConflicts}</h4>
    {conflicts.map((conflict, index) => <fieldset key={conflict.changeId}>
      <legend>{index + 1} · {zh[`editorialStructure_${conflict.kind}`]}</legend>
      <p>{conflict.kind === 'current_removed' ? zh.editorialRestoreHint : zh.editorialRemoveHint}</p>
      {conflict.currentModified && <p>{zh.editorialRemoveModified}</p>}
      {!!conflict.relatedNodes.length && <details><summary>{zh.editorialRelatedEffects} · {conflict.relatedNodes.length}</summary><ul>
        {conflict.relatedNodes.map((node, i) => <li key={node.nodeId}>{i + 1}. {zh[`editorialNode_${node.nodeType}`]} · {zh[`editorialRelated_${node.effect}`]}</li>)}
      </ul></details>}
      {!!conflict.preservedCommentCount && <p>{zh.editorialPreservedComments.replace('{count}',String(conflict.preservedCommentCount))}</p>}
      <div className="editorial-selection-actions">
        <Button size="small" disabled={disabled} aria-pressed={conflict.resolved} onClick={() => onChoose({changeId:conflict.changeId,choice:conflict.kind==='current_removed'?'restore_proposed':'remove_proposed'})}>{conflict.kind==='current_removed'?zh.editorialRestoreProposed:zh.editorialRemoveProposed}</Button>
        {canKeep && <Button size="small" disabled={disabled} onClick={() => onKeep(conflict.changeId)}>{zh.editorialKeepWithoutApplying}</Button>}
      </div>
    </fieldset>)}
  </section>;
}
