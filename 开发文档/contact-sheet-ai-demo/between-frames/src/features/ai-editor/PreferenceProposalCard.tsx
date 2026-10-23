import { useState } from 'react';
import type { PreferenceProposal } from '../../domain/agent';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

/** 对话建议经用户编辑并明确确认后，才成为本项目记忆。 */
export function PreferenceProposalCard({ proposal, disabled, onSave, onDismiss }: {
 proposal: PreferenceProposal; disabled: boolean; onSave: (statement: string) => Promise<void>; onDismiss: () => Promise<void>;
}) {
 const [text, setText] = useState(proposal.statement);
 const [pending, setPending] = useState(false);
 const act = async (save: boolean) => {
  if (pending || disabled) return;
  setPending(true);
  try { if (save) await onSave(text.trim()); else await onDismiss(); }
  finally { setPending(false); }
 };
 if (proposal.state === 'dismissed') return null;
 return <section className="chat-preference-card" aria-label={zh.chatPreference}>
  <strong>{zh.chatPreference}</strong>
  {proposal.state === 'pending' ? <textarea aria-label={zh.chatPreferenceText} maxLength={500} value={text} disabled={pending || disabled} onChange={event => setText(event.target.value)} /> : <p>{text}</p>}
  <p>{proposal.applicability}</p>
  <details><summary>{zh.chatPreferenceSource}</summary>{proposal.sources.map(source => <blockquote key={source.messageId}>{source.text}</blockquote>)}</details>
  {proposal.state === 'pending' ? <><p className="muted">{zh.chatPreferenceNotice}</p><div className="chat-preference-actions"><Button size="small" disabled={pending || disabled || !text.trim()} onClick={() => void act(true)}>{zh.chatPreferenceSave}</Button><Button size="small" disabled={pending || disabled} onClick={() => void act(false)}>{zh.chatPreferenceDismiss}</Button></div></> : <p role="status">{proposal.state === 'saved' ? zh.chatPreferenceSaved : zh.chatPreferenceRemoved}</p>}
 </section>;
}
