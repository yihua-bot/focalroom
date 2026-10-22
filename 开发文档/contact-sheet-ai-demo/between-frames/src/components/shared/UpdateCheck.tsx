// 自动更新：检查新版本 → 展示说明 → 下载校验 → 重启安装。
// 失败安静降级为"去官网下载"提示。
import { useState } from 'react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

type Phase = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'uptodate' | 'error';

export function UpdateCheck() {
  const [phase, setPhase] = useState<Phase>('idle');
  const [info, setInfo] = useState<{ version: string; notes: string } | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState('');

  const checkNow = async () => {
    setPhase('checking'); setError('');
    try {
      const update = await check();
      if (!update) { setPhase('uptodate'); return; }
      setInfo({ version: update.version, notes: update.body ?? '' });
      setPhase('available');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  };

  const downloadAndInstall = async () => {
    if (!info) return;
    setPhase('downloading');
    try {
      const update = await check();
      if (!update) { setPhase('uptodate'); return; }
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started' && event.data.contentLength) {
          setProgress(0);
        } else if (event.event === 'Progress') {
          setProgress(p => Math.min(99, p + 1));
        } else if (event.event === 'Finished') {
          setProgress(100);
        }
      });
      setPhase('ready');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  };

  const restart = async () => { await relaunch(); };

  if (phase === 'idle') {
    return <button className="update-check-btn" onClick={() => void checkNow()}>检查更新</button>;
  }
  if (phase === 'checking') return <span className="update-status">正在检查…</span>;
  if (phase === 'uptodate') return <span className="update-status ok">已是最新版</span>;
  if (phase === 'available' && info) {
    return (
      <div className="update-available" role="alert">
        <strong>新版本 {info.version} 可用</strong>
        {info.notes && <p className="update-notes">{info.notes}</p>}
        <button onClick={() => void downloadAndInstall()}>立即更新</button>
        <button className="quiet" onClick={() => setPhase('idle')}>稍后</button>
      </div>
    );
  }
  if (phase === 'downloading') {
    return (
      <div className="update-downloading">
        <span>下载中… {progress}%</span>
        <progress value={progress} max={100} />
      </div>
    );
  }
  if (phase === 'ready') {
    return (
      <div className="update-ready" role="status">
        <span>更新已就绪，重启后生效</span>
        <button onClick={() => void restart()}>重启应用</button>
      </div>
    );
  }
  return (
    <div className="update-error" role="alert">
      <span>检查更新失败：{error || '网络问题'}</span>
      <a href="https://focroom.cc/" target="_blank" rel="noopener">去官网下载</a>
    </div>
  );
}
