// Focal AI 自由对话：类 GPT 体验——直接聊天，不走任务引擎。
// 上下文自动注入（项目 brief + 选中照片）；AI 建议选片时一键切任务模式。
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { zh } from '../../app/copy';

interface Turn { role: 'user' | 'assistant'; content: string; }

export function FocalChat({ projectId, selectedPhotoIds, onSuggestTask, prefill }: {
  projectId: string;
  selectedPhotoIds: string[];
  onSuggestTask: (goal: string) => void;
  prefill?: string;
}) {
  const [turns, setTurns] = useState<Turn[]>([]);
  const [input, setInput] = useState(prefill ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => { scroller.current?.scrollTo({ top: 1e9 }); }, [turns]);

  const send = async () => {
    const message = input.trim();
    if (!message || busy) return;
    setBusy(true); setError(''); setInput('');
    const history = turns.slice(-20);
    setTurns(prev => [...prev, { role: 'user', content: message }]);
    try {
      const reply = await invoke<{ reply: string; suggestion: string }>('focal_chat', {
        cmd: { projectId, message, history, photoIds: selectedPhotoIds },
      });
      setTurns(prev => [...prev, { role: 'assistant', content: reply.reply }]);
      if (reply.suggestion === 'create_task') onSuggestTask(message);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  };

  return <div className="focal-chat">
    <div className="focal-chat-scroll" ref={scroller}>
      {turns.length === 0 && <div className="focal-chat-hint">
        <p>问我任何关于这组照片的问题——</p>
        <p className="muted">「这批照片适合什么主题的黑白处理？」<br/>「帮我看看哪几张构图最强」<br/>「选出 20 张做展览」</p>
      </div>}
      {turns.map((t, i) => (
        <div key={i} className={`focal-chat-msg ${t.role}`}>
          {t.role === 'assistant' && <span className="focal-chat-avatar">F</span>}
          <div className="focal-chat-bubble">{t.content}</div>
        </div>
      ))}
      {busy && <div className="focal-chat-msg assistant"><span className="focal-chat-avatar">F</span><div className="focal-chat-bubble thinking">…</div></div>}
      {error && <p role="alert" className="fr-error">{error}</p>}
    </div>
    <div className="focal-chat-input">
      <textarea
        value={input}
        placeholder="问 Focal AI…（Enter 发送，Shift+Enter 换行）"
        rows={2}
        maxLength={4000}
        disabled={busy}
        onChange={e => setInput(e.target.value)}
        onKeyDown={e => {
          if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); }
        }}
      />
      <button className="send" disabled={busy || !input.trim()} onClick={() => void send()} aria-label={zh.send}>↑</button>
    </div>
  </div>;
}
