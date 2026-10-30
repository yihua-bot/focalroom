// 自动更新：启动时静默检查 → 发现新版自动弹浮窗 → 一键更新 → 自动重启。
// 不再藏在帮助对话框里。
import { useEffect, useRef, useState } from 'react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke, isTauri } from '@tauri-apps/api/core';

type Phase = 'idle' | 'downloading' | 'ready' | 'error';

/** 桌面端经系统浏览器打开官网；裸 target=_blank 在 Tauri WebView 中无响应。 */
function WebsiteLink({ children }: { children: React.ReactNode }) {
  const [failed, setFailed] = useState(false);
  return <>
    <a href="https://focroom.cc/" target="_blank" rel="noopener" onClick={event => {
      if (!isTauri()) return;
      event.preventDefault();
      setFailed(false);
      void invoke('open_product_link', { link: 'website' }).catch(() => setFailed(true));
    }}>{children}</a>
    {failed && <span> 无法打开系统浏览器，请复制 focroom.cc 到浏览器访问</span>}
  </>;
}

/** 启动时自动检查更新；有新版时显示顶部浮窗。 */
export function AutoUpdateBanner() {
  const [update, setUpdate] = useState<{ version: string; notes: string } | null>(null);
  const [phase, setPhase] = useState<Phase>('idle');
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState('');
  const [dismissed, setDismissed] = useState(false);
  const started = useRef(false);

  // 启动 3 秒后静默检查（不阻塞加载）
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    const timer = setTimeout(async () => {
      try {
        const result = await check();
        if (result) {
          setUpdate({ version: result.version, notes: result.body ?? '' });
        }
      } catch { /* 静默失败——网络问题不打扰用户 */ }
    }, 3000);
    return () => clearTimeout(timer);
  }, []);

  const doUpdate = async () => {
    if (phase === 'downloading') return;
    setPhase('downloading'); setError(''); setProgress(0);
    try {
      const result = await check();
      if (!result) { setPhase('idle'); setUpdate(null); return; }
      await result.downloadAndInstall((event) => {
        if (event.event === 'Started' && event.data.contentLength) {
          setProgress(1);
        } else if (event.event === 'Progress') {
          setProgress(p => Math.min(99, p + 2));
        } else if (event.event === 'Finished') {
          setProgress(100);
        }
      });
      setPhase('ready');
      // 自动重启（1 秒后让用户看到完成状态）
      setTimeout(() => { void relaunch(); }, 1000);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('error');
    }
  };

  if (!update || dismissed) return null;

  // 下载中/完成：不可关闭的浮窗
  if (phase === 'downloading' || phase === 'ready') {
    return <div className="update-banner active" role="alert">
      <div className="update-banner-inner">
        <span className="update-banner-icon" aria-hidden>⬇️</span>
        <div className="update-banner-body">
          {phase === 'downloading' ? <>
            <span>正在下载 v{update.version}… {progress}%</span>
            <progress value={progress} max={100} className="update-progress" />
          </> : <span>✅ 更新完成，正在重启…</span>}
        </div>
      </div>
    </div>;
  }

  // 有新版：顶部浮窗（可关闭稍后更新）
  return <div className="update-banner" role="status">
    <div className="update-banner-inner">
      <span className="update-banner-icon" aria-hidden>✨</span>
      <div className="update-banner-body">
        <strong>新版本 v{update.version} 可用</strong>
        {update.notes && <p className="update-banner-notes">{update.notes.split('\n')[0].slice(0, 80)}</p>}
      </div>
      <div className="update-banner-actions">
        <button className="update-btn-primary" onClick={() => void doUpdate()}>立即更新</button>
        <button className="update-btn-later" onClick={() => setDismissed(true)}>稍后</button>
      </div>
    </div>
    {phase === 'error' && <p className="update-banner-error">更新失败：{error} · <WebsiteLink>去官网下载</WebsiteLink></p>}
  </div>;
}

/** 手动检查（保留在帮助对话框中的入口） */
export function UpdateCheck() {
  const [phase, setPhase] = useState<'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'uptodate' | 'error'>('idle');
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
        if (event.event === 'Progress') setProgress(p => Math.min(99, p + 2));
        else if (event.event === 'Finished') setProgress(100);
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
        {info.notes && <p className="update-available-notes">{info.notes}</p>}
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
    <div className="update-error-inline" role="alert">
      <span>检查更新失败：{error || '网络问题'}</span>
      <WebsiteLink>去官网下载</WebsiteLink>
    </div>
  );
}
