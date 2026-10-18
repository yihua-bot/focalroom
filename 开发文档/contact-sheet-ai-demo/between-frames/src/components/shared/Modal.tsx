import * as Dialog from '@radix-ui/react-dialog';
import { X } from 'lucide-react';
import type { ReactNode, RefObject } from 'react';
import { useState } from 'react';
import { zh } from '../../app/copy';

export function Modal({ title, description, children, onClose, wide = false, initialFocusRef }: {
  title: string; description?: string; children: ReactNode; onClose: () => void; wide?: boolean;
  initialFocusRef?: RefObject<HTMLElement | null>;
}) {
  const [returnFocus] = useState(() => document.activeElement instanceof HTMLElement ? document.activeElement : null);
  return <Dialog.Root open onOpenChange={open => { if (!open) onClose(); }}>
    <Dialog.Portal><Dialog.Overlay className="modal-overlay" />
      <Dialog.Content className={`modal ${wide ? 'modal-wide' : ''}`} {...(!description ? { 'aria-describedby': undefined } : {})} onOpenAutoFocus={event => { if (initialFocusRef?.current) { event.preventDefault(); initialFocusRef.current.focus(); } }} onCloseAutoFocus={event => { event.preventDefault(); if (returnFocus?.isConnected) returnFocus.focus(); }}>
        <div className="modal-head"><div>
          <Dialog.Title>{title}</Dialog.Title></div>
          <Dialog.Close className="icon-button" aria-label={zh.close}><X size={20} /></Dialog.Close></div>
        {description && <Dialog.Description className="muted">{description}</Dialog.Description>}
        {children}
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>;
}
