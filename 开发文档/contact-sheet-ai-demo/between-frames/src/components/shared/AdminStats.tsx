// 管理员统计面板：注册量 + 下载量实时展示（admin 账号可见）。
// 挂载在项目列表底部；仅在管理员登录时渲染。
import { useEffect, useState } from 'react';
import { useAccount } from '../../features/cloud-login/AccountProvider';

interface StatsData {
  registrations: { total: number; verified: number };
  downloads: Record<string, number>;
  downloadsTotal: number;
}

/** 从 Rust 后端拿云会话 token（不暴露给前端日志/模型）。 */
async function cloudToken(): Promise<string | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const token = await invoke<string | null>('cloud_session_token');
    return token;
  } catch { return null; }
}

export function AdminStats() {
  const account = useAccount();
  const isAdmin = account?.status?.user?.email === 'admin20251018@focroom.cc';

  const [data, setData] = useState<StatsData | null>(null);
  useEffect(() => {
    if (!isAdmin) return;
    let active = true;
    const load = async () => {
      try {
        const token = await cloudToken();
        const res = await fetch('https://team.focroom.cc/api/v2/stats/overview', {
          headers: token ? { authorization: `Bearer ${token}` } : {},
        });
        if (res.ok && active) setData(await res.json());
      } catch { /* 网络失败保持上次数据 */ }
    };
    void load();
    const timer = setInterval(load, 30_000);
    return () => { active = false; clearInterval(timer); };
  }, [isAdmin]);

  if (!isAdmin) return null;

  const d = data;
  return <div className="admin-stats" aria-label="产品统计">
    <h4>产品统计</h4>
    <div className="admin-stats-grid">
      <div className="stat-card">
        <span className="stat-number">{d?.registrations.total ?? '…'}</span>
        <span className="stat-label">注册用户</span>
        <span className="stat-sub">已验证 {d?.registrations.verified ?? '–'}</span>
      </div>
      <div className="stat-card">
        <span className="stat-number">{d?.downloadsTotal ?? '…'}</span>
        <span className="stat-label">产品下载</span>
        <span className="stat-sub">DMG {d?.downloads?.dmg ?? '–'}</span>
      </div>
    </div>
  </div>;
}
