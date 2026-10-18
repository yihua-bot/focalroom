import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from '../../components/shared/Button';
import { useI18n } from '../../i18n';

interface DeepLinkEvent {
  path: string;
  params: Record<string, string>;
}

export function CloudLoginView({ onLogin, onClose }: {
  onLogin: (token: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [waiting, setWaiting] = useState(false);
  const [error, setError] = useState('');
  const [loginUrl, setLoginUrl] = useState('');

  useEffect(() => {
    // 监听深链回调
    const unlisten = listen<DeepLinkEvent>('deeplink', async (event) => {
      const { path, params } = event.payload;
      if (path === 'callback' || path === 'auth') {
        const token = params.token ?? '';
        const state = params.state ?? '';
        if (token && state) {
          try {
            const ok = await invoke('cloud_login_callback', { token, state });
            if (ok) {
              onLogin(token);
              setWaiting(false);
            }
          } catch (e) {
            setError(String(e));
            setWaiting(false);
          }
        }
      }
    });
    return () => { void unlisten.then(fn => fn()); };
  }, [onLogin]);

  const startLogin = async () => {
    setError('');
    setWaiting(true);
    try {
      const baseUrl = 'https://team.focroom.cc/shared';
      const url = await invoke<string>('cloud_login_open', { baseUrl });
      setLoginUrl(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setWaiting(false);
    }
  };

  return (
    <div style={{ display:'flex', height:'100%', alignItems:'center', justifyContent:'center' }}>
      <div style={{ width:380, borderRadius:16, border:'1px solid var(--border,#333)', padding:32, background:'var(--card-bg,#1a1a2e)' }}>
        <div style={{ marginBottom: 16, display:'flex', alignItems:'center', gap:12 }}>
          <span style={{ fontSize: 36 }}>📸</span>
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>FocalRoom</div>
            <div style={{ fontSize: 12, color:'var(--text2,#888)' }}>{t('auth.login')} · Cloud</div>
          </div>
        </div>

        <p style={{ margin:'24px 0', fontSize: 13, color:'var(--text2,#aaa)', lineHeight: 1.5 }}>
          点击按钮在系统浏览器打开登录页。支持验证码、Google 等所有登录方式。
          登录成功后会自动返回这里。
        </p>

        {waiting ? (
          <>
            <div style={{ textAlign:'center', padding:'20px 0' }}>
              <div style={{ fontSize: 14, color:'var(--accent,#6c5ce7)' }}>
                ⏳ 等待浏览器登录回传…
              </div>
              <div style={{ fontSize: 11, color:'var(--text2,#666)', marginTop: 8 }}>
                如果浏览器没有自动跳转，请手动复制登录地址：
              </div>
              <div style={{ fontSize: 10, color:'var(--text2,#555)', marginTop: 4, wordBreak:'break-all', fontFamily:'monospace' }}>
                {loginUrl}
              </div>
            </div>
          </>
        ) : (
          <Button variant="primary" onClick={() => void startLogin()} style={{ width:'100%' }}>
            🌐 打开浏览器登录
          </Button>
        )}

        {error && (
          <p role="alert" style={{ color:'#e74c3c', fontSize: 12, marginTop: 12 }}>{error}</p>
        )}

        <button
          onClick={onClose}
          style={{ marginTop: 16, width:'100%', background:'none', border:'none', color:'var(--text2,#666)', fontSize: 12, cursor:'pointer' }}
        >
          暂不登录，直接使用本地功能
        </button>
      </div>
    </div>
  );
}
