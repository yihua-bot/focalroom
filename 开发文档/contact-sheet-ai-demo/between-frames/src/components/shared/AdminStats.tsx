// 管理员统计面板：注册量 + 下载量实时展示（admin 账号可见）。
import { useQuery } from '@tanstack/react-query';
import { useAccount } from '../../features/cloud-login/AccountProvider';

interface StatsData {
  registrations: { total: number; verified: number };
  downloads: Record<string, number>;
  downloadsTotal: number;
}

export function AdminStats() {
  const account = useAccount();
  const isAdmin = account?.status?.user?.email === 'admin20251018@focroom.cc';

  const stats = useQuery<StatsData>({
    queryKey: ['admin-stats'],
    queryFn: async () => {
      const res = await fetch('https://team.focroom.cc/api/v2/stats/overview', {
      });
      if (!res.ok) throw new Error('统计获取失败');
      return res.json();
    },
    enabled: isAdmin,
    refetchInterval: 30_000, // 30 秒自动刷新
  });

  if (!isAdmin) return null;

  const d = stats.data;
  return (
    <div className="admin-stats" aria-label="产品统计">
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
          <span className="stat-sub">DMG {d?.downloads?.dmg ?? '–'} · 更新 {d?.downloads?.updater ?? 0}</span>
        </div>
      </div>
    </div>
  );
}
