import { useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';

export function SharedReviewLink({ projectId, url }: { projectId: string; url: string }) {
  const [failed, setFailed] = useState(false);
  return <><a href={url} target="_blank" rel="noreferrer" onClick={event => {
    if (!isTauri()) return;
    event.preventDefault();
    setFailed(false);
    void invoke('open_shared_review', { projectId }).catch(() => setFailed(true));
  }}>打开共享评片 ↗</a>{failed && <span role="alert">无法打开系统浏览器，请复制此地址到浏览器：{url}</span>}</>;
}
