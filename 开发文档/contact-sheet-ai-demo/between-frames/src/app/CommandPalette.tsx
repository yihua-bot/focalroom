import { useEffect, useMemo, useRef, useState } from 'react';
import { Modal } from '../components/shared/Modal';

export interface CommandEntry {
  id: string;
  label: string;
  hint?: string;
  group: string;
  run: () => void;
}

/** ⌘K 命令面板：输入过滤、上下选择、Enter 执行；Esc 关闭由 Modal 处理。 */
export function CommandPalette({ commands, onClose }: { commands: CommandEntry[]; onClose: () => void }) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const filtered = useMemo(() => {
    const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    return commands.filter(command => terms.every(term => `${command.group} ${command.label}`.toLowerCase().includes(term)));
  }, [commands, query]);
  useEffect(() => {
    const item = listRef.current?.children[active];
    item?.scrollIntoView?.({ block: 'nearest' });
  }, [active]);
  const execute = (index: number) => {
    const command = filtered[index];
    if (!command) return;
    onClose();
    command.run();
  };
  return <Modal title="命令面板" description="输入关键词过滤，↑↓ 选择，Enter 执行。" onClose={onClose}>
    <input
      ref={inputRef}
      className="command-palette-input"
      aria-label="搜索命令"
      placeholder="搜索命令…"
      value={query}
      autoFocus
      onChange={event => { setQuery(event.target.value); setActive(0); }}
      onKeyDown={event => {
        if (event.key === 'ArrowDown') { event.preventDefault(); setActive(value => Math.min(value + 1, filtered.length - 1)); }
        if (event.key === 'ArrowUp') { event.preventDefault(); setActive(value => Math.max(value - 1, 0)); }
        if (event.key === 'Enter') { event.preventDefault(); execute(active); }
      }}
    />
    <ul ref={listRef} className="command-palette-list" role="listbox" aria-label="命令列表">
      {filtered.map((command, index) => <li key={command.id} role="option" aria-selected={index === active}
        onMouseEnter={() => setActive(index)}
        onClick={() => execute(index)}>
        <span className="command-group">{command.group}</span>
        <span className="command-label">{command.label}</span>
        {command.hint && <kbd className="command-hint">{command.hint}</kbd>}
      </li>)}
      {!filtered.length && <li className="command-empty" role="option" aria-selected="false" aria-disabled="true">没有匹配的命令</li>}
    </ul>
  </Modal>;
}
