import { useState } from 'react';
import { ExternalLink, Mail, SlidersHorizontal } from 'lucide-react';
import type { DiagnosticsRepository } from '../domain/diagnostics';
import { LanguageSwitcher } from '../i18n';
import { Modal } from '../components/shared/Modal';
import { Button } from '../components/shared/Button';
import { DiagnosticsDialog } from '../features/diagnostics/DiagnosticsDialog';
import { invoke } from '@tauri-apps/api/core';
import { desktopRuntime } from '../services/TauriRepository';
import { zh } from './copy';

export function SettingsDialog({ page, diagnostics, onClose }: {
  page: 'general' | 'contact'; diagnostics?: DiagnosticsRepository; onClose: () => void;
}) {
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState('');
  const openLink = (event: React.MouseEvent, link: 'website' | 'instagram') => {
    if (!desktopRuntime()) return;
    event.preventDefault();
    void invoke('open_product_link', { link }).catch(() => setError(zh.browserOpenFailed));
  };
  const email = 'sliqiang85@gmail.com';
  return <Modal title={page === 'contact' ? zh.contactAuthor : zh.settings} onClose={onClose}>
    {page === 'general' ? <div className="global-settings">
      <section className="settings-section"><h3>{zh.generalSettings}</h3><div className="settings-row"><label>{zh.language}</label><LanguageSwitcher /></div></section>
      {diagnostics && <section className="settings-section"><h3>{zh.storageSettings}</h3><Button onClick={() => setShowDiagnostics(true)}><SlidersHorizontal size={15} />{zh.storageDetails}</Button></section>}
      <section className="settings-section settings-about"><img src="/focalroom-mark.png" alt="" /><div><strong>FocalRoom</strong><a onClick={event => openLink(event, 'website')} href="https://focroom.cc" target="_blank" rel="noreferrer">focroom.cc <ExternalLink size={12} /></a></div></section>
    </div> : <div className="contact-author">
      <div className="contact-avatar">Y</div><h3>Yihua</h3>
      <div className="contact-card"><Mail size={17} /><span>{email}</span><Button size="small" onClick={() => { setError(''); void navigator.clipboard.writeText(email).then(() => setCopied(true)).catch(() => setError(zh.copyFailed)); }}>{copied ? zh.copied : zh.copyEmail}</Button></div>
      <a className="contact-card" onClick={event => openLink(event, 'instagram')} href="https://www.instagram.com/YIHUAPHOTO/" target="_blank" rel="noreferrer"><span>Instagram</span><strong>@YIHUAPHOTO</strong><ExternalLink size={15} /></a>
    </div>}
    {error && <p role="alert">{error}</p>}
    {showDiagnostics && diagnostics && <DiagnosticsDialog repository={diagnostics} onClose={() => setShowDiagnostics(false)} />}
  </Modal>;
}
