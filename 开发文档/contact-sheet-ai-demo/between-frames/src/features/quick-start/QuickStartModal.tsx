import { useState } from 'react';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';

/** 从自然语言描述提取项目参数（本地规则，不调 AI）。 */
export function parseGoal(text: string): { title: string; count: number; goal: string } {
  const trimmed = text.trim();
  // 提取数量：中文数字或阿拉伯数字。
  const countMatch = trimmed.match(/(\d+)\s*张/) ?? trimmed.match(/选[出择]?(\d+)/);
  const count = countMatch ? Math.min(Math.max(parseInt(countMatch[1], 10), 1), 100) : 20;
  // 提取主题：第一句去掉「我要/帮我/制作/做一个」等前缀。
  const topic = trimmed
    .replace(/^我想?\s*[要做]|^帮我\s*[做创建]|^制作\s*一?[个本]?|^做一个?|^创建\s*一?[个本]?/g, '')
    .replace(/^关于/, '')
    .split(/[。.!！?？\n]/)[0]
    .trim();
  const title = topic.length > 2 ? topic.slice(0, 40) : '新项目';
  // 完整原文作为 Agent 目标。
  const goal = trimmed.slice(0, 1500);
  return { title, count, goal };
}

export function QuickStartModal({ onClose, onCreate, busy = false }: {
  onClose: () => void;
  onCreate: (title: string, goal: string, count: number) => void;
  busy?: boolean;
}) {
  const [text, setText] = useState('');
  const [step, setStep] = useState<'describe' | 'confirm'>('describe');
  const parsed = text.trim() ? parseGoal(text) : null;

  const start = () => {
    if (!parsed) return;
    setStep('confirm');
  };

  const create = () => {
    if (!parsed || busy) return;
    onCreate(parsed.title, parsed.goal, parsed.count);
  };

  return <Modal title={zh["quickstart.title"]} onClose={() => { if (!busy) onClose(); }} initialFocusRef={undefined}>
    {step === 'describe' && <>
      <div className="form-group">
        <label htmlFor="quick-start-input">{zh["quickstart.input.label"]}</label>
        <textarea
          id="quick-start-input"
          rows={6}
          maxLength={1500}
          placeholder={zh["quickstart.placeholder"]}
          value={text}
          onChange={e => setText(e.target.value)}
          autoFocus
        />
      </div>
      {parsed && <div className="quick-start-preview">
        <p><strong>{zh["quickstart.preview.title"]}：</strong>{parsed.title}</p>
        <p><strong>{zh["quickstart.preview.count"]}：</strong>{parsed.count} 张</p>
        <p><strong>{zh["quickstart.preview.goal"]}：</strong>{parsed.goal.slice(0, 100)}{parsed.goal.length > 100 ? '…' : ''}</p>
      </div>}
      <div className="dialog-actions">
        <Button variant="primary" disabled={!parsed} onClick={start}>{zh["quickstart.next"]}</Button>
      </div>
    </>}
    {step === 'confirm' && parsed && <>
      <div className="quick-start-preview">
        <p><strong>{zh["quickstart.preview.title"]}：</strong>「{parsed.title}」</p>
        <p><strong>{zh["quickstart.preview.count"]}：</strong>{parsed.count} 张</p>
        <p><strong>{zh["quickstart.preview.goal"]}：</strong>{parsed.goal}</p>
      </div>
      <div className="dialog-actions">
        <Button disabled={busy} onClick={() => setStep('describe')}>{zh["quickstart.back"]}</Button>
        <Button variant="primary" disabled={busy} onClick={create}>{zh["quickstart.confirm"]}</Button>
      </div>
    </>}
  </Modal>;
}
