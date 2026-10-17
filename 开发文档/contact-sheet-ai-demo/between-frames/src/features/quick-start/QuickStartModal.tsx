import { useState } from 'react';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';

/** 从自然语言描述提取项目参数（本地规则，不调 AI）。 */
export function parseGoal(text: string): { title: string; count: number; goal: string } {
  const trimmed = text.trim();
  // 提取数量：中文数字或阿拉伯数字。
  const countMatch = trimmed.match(/(\d+)\s*张/) ?? trimmed.match(/选[出择]?(\d+)/);
  const count = countMatch ? Math.min(Math.max(parseInt(countMatch[1], 10), 1), 500) : 20;
  // 提取主题：第一句去掉「我要/帮我/制作/做一个」等前缀。
  const topic = trimmed
    .replace(/^我想?\s*[要做]|^帮我\s*[做创建]|^制作\s*一?[个本]?|^做一个?|^创建\s*一?[个本]?/g, '')
    .replace(/^关于/, '')
    .split(/[。.!！?？\n]/)[0]
    .trim();
  const title = topic.length > 2 ? topic.slice(0, 40) : '新项目';
  // 完整原文作为 Agent 目标。
  const goal = trimmed.slice(0, 4000);
  return { title, count, goal };
}

export function QuickStartModal({ onClose, onCreate, onImport }: {
  onClose: () => void;
  onCreate: (title: string, goal: string, count: number) => void;
  onImport: () => void;
}) {
  const [text, setText] = useState('');
  const [step, setStep] = useState<'describe' | 'confirm'>('describe');
  const parsed = text.trim() ? parseGoal(text) : null;

  const start = () => {
    if (!parsed) return;
    setStep('confirm');
  };

  const create = () => {
    if (!parsed) return;
    onCreate(parsed.title, parsed.goal, parsed.count);
    onClose();
    onImport();
  };

  return <Modal title="快速开始" description="描述你想做的项目，我会帮你设置好一切" onClose={() => { onClose(); setStep('describe'); }} initialFocusRef={undefined}>
    {step === 'describe' && <>
      <div className="form-group">
        <label htmlFor="quick-start-input">告诉我你想做什么</label>
        <textarea
          id="quick-start-input"
          rows={4}
          placeholder="例：我要制作一本关于夏日家庭生活的摄影小书，从照片中选出 20 张，突出自然互动与时间感..."
          value={text}
          onChange={e => setText(e.target.value)}
          autoFocus
        />
      </div>
      {parsed && <div className="quick-start-preview">
        <p><strong>项目名称：</strong>{parsed.title}</p>
        <p><strong>目标张数：</strong>{parsed.count} 张</p>
        <p><strong>AI 目标：</strong>{parsed.goal.slice(0, 100)}{parsed.goal.length > 100 ? '…' : ''}</p>
      </div>}
      <div className="dialog-actions">
        <Button variant="primary" disabled={!parsed} onClick={start}>下一步</Button>
      </div>
    </>}
    {step === 'confirm' && parsed && <>
      <p>我理解你的需求如下：</p>
      <div className="quick-start-preview">
        <p><strong>📝 项目：</strong>「{parsed.title}」</p>
        <p><strong>🎯 选出：</strong>{parsed.count} 张</p>
        <p><strong>🤖 AI 目标：</strong>{parsed.goal}</p>
      </div>
      <p className="micro">创建后我会打开导入窗口，你选择照片文件夹。导入完成后打开「AI 建议」面板，目标已预填，点开始即可。</p>
      <div className="dialog-actions">
        <Button onClick={() => setStep('describe')}>← 修改描述</Button>
        <Button variant="primary" onClick={create}>创建项目并导入照片 →</Button>
      </div>
    </>}
  </Modal>;
}
