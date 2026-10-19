import * as Dialog from '@radix-ui/react-dialog';
import { Settings2, Sparkles, X } from 'lucide-react';
import { useRef, useState, type ReactNode } from 'react';
import { zh } from '../../app/copy';

export function AISelectionDrawer({ projectTitle, onClose, children }: {
  projectTitle: string; onClose: () => void;
  children: (settingsOpen: boolean, closeSettings: () => void) => ReactNode;
}) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const opener = useRef(document.activeElement as HTMLElement | null);
  const content = useRef<HTMLDivElement>(null);
  return <Dialog.Root open modal={false} onOpenChange={open => { if (!open) onClose(); }}>
    <Dialog.Portal><Dialog.Content ref={content} className="ai-drawer" aria-describedby={undefined}
      onCloseAutoFocus={event => { event.preventDefault(); if (opener.current?.isConnected) opener.current.focus(); }}
      onInteractOutside={event => event.preventDefault()}
      onOpenAutoFocus={event => { event.preventDefault(); content.current?.querySelector('textarea')?.focus(); }}>
      <header className="ai-drawer-header">
        <span className="ai-orb"><Sparkles size={19} /></span>
        <div><Dialog.Title>{zh.ai}</Dialog.Title><p>{projectTitle}</p></div>
        <button className="icon-button" aria-label={zh.aiSettings} onClick={() => setSettingsOpen(true)}><Settings2 size={18} /></button>
        <Dialog.Close className="icon-button" aria-label={zh.closeAI}><X size={19} /></Dialog.Close>
      </header>
      {children(settingsOpen, () => setSettingsOpen(false))}
    </Dialog.Content></Dialog.Portal>
  </Dialog.Root>;
}
