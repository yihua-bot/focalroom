// Focal AI 自由对话：类 GPT 体验——直接聊天，不走任务引擎。
// 上下文自动注入（项目 brief + 选中照片）；AI 建议选片时一键切任务模式。
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { zh } from '../../app/copy';

interface Turn { role: 'user' | 'assistant'; content: string; }

export function FocalChat({ projectId, selectedPhotoIds, photoNames, onSuggestTask, prefill }: {
  projectId: string;
  selectedPhotoIds: string[];
  photoNames?: Record<string, string>;
  onSuggestTask: (goal: string) => void;
  prefill?: string;
}) {
  const storageKey = `focal-chat:${projectId}`;
  const [turns, setTurns] = useState<Turn[]>(() => {
    try { return JSON.parse(localStorage.getItem(storageKey) ?? '[]') as Turn[]; } catch { return []; }
  });
  const [input, setInput] = useState(prefill ?? '');
  const [busy, setBusy] = useState(false);
  const [includeImages, setIncludeImages] = useState(false);
  const [error, setError] = useState('');
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scroller.current?.scrollTo({ top: 1e9 });
    try { localStorage.setItem(storageKey, JSON.stringify(turns.slice(-100))); } catch { /* 存储满静默 */ }
  }, [turns, storageKey]);

  const send = async () => {
    const message = input.trim();
    if (!message || busy) return;
    setBusy(true); setError(''); setInput('');
    const history = turns.slice(-20);
    setTurns(prev => [...prev, { role: 'user', content: message }]);
    try {
      const reply = await invoke<{ reply: string; suggestion: string }>('focal_chat', {
        cmd: { projectId, message, history, photoIds: selectedPhotoIds, includeImages },
      });
      setTurns(prev => [...prev, { role: 'assistant', content: reply.reply }]);
      if (reply.suggestion === 'create_task') onSuggestTask(message);
      else if (reply.suggestion === 'analyze_photos' && selectedPhotoIds.length > 0) setIncludeImages(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally { setBusy(false); }
  };

  const renderWithPhotoRefs = (text: string, names?: Record<string, string>) => {
    if (!names || Object.keys(names).length === 0) return text;
    // 匹配照片文件名（如 IMG_2041.jpg、DSC_0042.png 等）
    const parts = text.split(/([A-Za-z0-9_-]+\.(?:jpg|jpeg|png|webp|tif|tiff|heic|dng|cr2|cr3|nef|arw))/gi);
    return parts.map((part, i) => {
      const matched = Object.entries(names).find(([, name]) => name === part);
      if (matched) return <span key={i} className="focal-photo-ref" title={matched[0]}>{part}</span>;
      return part;
    });
  };

  const costEstimate = (images: number) => images === 0 ? '2–5 千' : `${2 + images * 15}–${5 + images * 30} 千`;

  const clearChat = () => { setTurns([]); try { localStorage.removeItem(storageKey); } catch { /* 忽略 */ } };

  return <div className="focal-chat">
    <div className="focal-chat-scroll" ref={scroller}>
      {turns.length === 0 && <div className="focal-chat-hint">
        <p>问我任何关于这组照片的问题——</p>
        <p className="muted">「这批照片适合什么主题的黑白处理？」<br/>「帮我看看哪几张构图最强」<br/>「选出 20 张做展览」</p>
      </div>}
      {turns.map((t, i) => (
        <div key={i} className={`focal-chat-msg ${t.role}`}>
          {t.role === 'assistant' && <span className="focal-chat-avatar">F</span>}
          <div className="focal-chat-bubble">{renderWithPhotoRefs(t.content, photoNames)}</div>
        </div>
      ))}
      {busy && <div className="focal-chat-msg assistant"><span className="focal-chat-avatar">F</span><div className="focal-chat-bubble thinking">…</div></div>}
      {error && <p role="alert" className="fr-error">{error}</p>}
    </div>
    {turns.length > 0 && <div className="focal-chat-meta">
      <span className="cost">{includeImages && selectedPhotoIds.length > 0 ? `本次约 ${costEstimate(Math.min(selectedPhotoIds.length, 6))} tokens` : `文字对话约 ${costEstimate(0)} tokens/次`}</span>
      <button className="clear" onClick={clearChat} title="清空对话">清空</button>
    </div>}
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
      <button className={`attach ${includeImages ? 'on' : ''}`} disabled={busy || selectedPhotoIds.length === 0}
        onClick={() => setIncludeImages(!includeImages)}
        title={includeImages ? `将发送 ${Math.min(selectedPhotoIds.length, 6)} 张照片预览（约 ${costEstimate(Math.min(selectedPhotoIds.length, 6))} tokens）` : '附加选中照片（最多 6 张）'}>📷{selectedPhotoIds.length > 0 && <span className="count">{Math.min(selectedPhotoIds.length, 6)}</span>}</button>
      <button className="send" disabled={busy || !input.trim()} onClick={() => void send()} aria-label={zh.send}>↑</button>
    </div>
  </div>;
}
