import { useState } from 'react';
import { ArrowUpRight, Check, Image, LockKeyhole } from 'lucide-react';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import type { CloudAuthRepository, CloudStatus } from '../../domain/cloud-auth';
import { useI18n } from '../../i18n';

export type LoginReason = 'account' | 'ai' | 'sharing' | 'subscription';
const text = {
  zh: { title: '登录 FocalRoom', account: '让每一次创作，都有自己的位置。', ai: '登录后即可使用 Focal AI，查看可用额度并管理订阅。', sharing: '登录后可邀请他人审稿，管理访问权限，并保留讨论与修改记录。', subscription: '登录后可选择订阅、查看剩余用量和管理账单。', local: '本地照片与项目仍保留在你的电脑上。', privacy: '登录不会自动上传照片。', action: '继续登录', later: '暂不登录', waiting: '请在浏览器完成登录', waitingHint: '完成后会自动返回这里，你可以继续刚才的操作。', cancel: '取消登录', done: '登录成功', browser: '将在浏览器中安全登录', unavailable: '请在 FocalRoom 桌面版登录。', error: '无法打开登录页面，请重试。' },
  en: { title: 'Sign in to FocalRoom', account: 'A space for every creative project.', ai: 'Sign in to use AI selection, check your allowance, and manage your plan.', sharing: 'Sign in to invite reviewers, manage access, and keep discussions and changes together.', subscription: 'Sign in to choose a plan, check usage, and manage billing.', local: 'Your local photos and projects stay on your computer.', privacy: 'Signing in never uploads photos automatically.', action: 'Continue to sign in', later: 'Not now', waiting: 'Complete sign-in in your browser', waitingHint: 'You’ll return here automatically to continue.', cancel: 'Cancel sign-in', done: 'Signed in', browser: 'Sign in securely in your browser', unavailable: 'Please sign in using the FocalRoom desktop app.', error: 'Could not open sign-in. Please try again.' },
  ja: { title: 'FocalRoom にログイン', account: 'すべての制作に、自分の場所を。', ai: 'ログインして AI セレクトを利用し、利用状況とプランを管理できます。', sharing: 'ログインしてレビュー担当者を招待し、アクセスやコメントを管理できます。', subscription: 'ログインしてプラン、利用状況、お支払いを管理できます。', local: 'ローカルの写真とプロジェクトはこのコンピュータに保存されます。', privacy: 'ログインだけで写真がアップロードされることはありません。', action: 'ログインに進む', later: 'あとで', waiting: 'ブラウザでログインを完了してください', waitingHint: '完了すると自動でここに戻り、操作を続けられます。', cancel: 'ログインを中止', done: 'ログインしました', browser: 'ブラウザで安全にログインします', unavailable: 'FocalRoom デスクトップアプリでログインしてください。', error: 'ログイン画面を開けませんでした。もう一度お試しください。' },
  ko: { title: 'FocalRoom 로그인', account: '모든 창작 프로젝트를 위한 나만의 공간.', ai: '로그인하여 Focal AI을 사용하고 이용량과 구독을 관리하세요.', sharing: '로그인하여 검토자를 초대하고 접근 권한과 의견을 관리하세요.', subscription: '로그인하여 구독, 이용량 및 결제 내역을 관리하세요.', local: '로컬 사진과 프로젝트는 컴퓨터에 보관됩니다.', privacy: '로그인해도 사진이 자동으로 업로드되지 않습니다.', action: '로그인 계속하기', later: '나중에', waiting: '브라우저에서 로그인을 완료하세요', waitingHint: '완료 후 자동으로 돌아와 작업을 계속할 수 있습니다.', cancel: '로그인 취소', done: '로그인 완료', browser: '브라우저에서 안전하게 로그인합니다', unavailable: 'FocalRoom 데스크톱 앱에서 로그인하세요.', error: '로그인 화면을 열 수 없습니다. 다시 시도하세요.' },
} as const;

export function LoginDialog({ repository, status, reason, onClose, onStarted }: { repository?: CloudAuthRepository; status: CloudStatus | null; reason: LoginReason; onClose: () => void; onStarted?: () => void }) {
  const { locale } = useI18n();
  const copy = text[locale];
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const login = async () => {
    if (!repository || busy) return;
    setBusy(true); setError('');
    try { await repository.open(); onStarted?.(); }
    catch { setError(copy.error); }
    finally { setBusy(false); }
  };
  const close = () => {
    if (busy) return;
    if (status?.waiting) void repository?.cancel().catch(() => {});
    onClose();
  };
  return <Modal title={copy.title} description={copy[reason]} onClose={close}>
    <div className="login-prompt">
      <img src="/focalroom-mark.png" alt="" className="login-prompt-mark" />
      <div className="login-benefit"><Image size={17} /><span>{copy.local}</span></div>
      <div className="login-benefit"><LockKeyhole size={17} /><span>{copy.privacy}</span></div>
      {status?.authenticated ? <p role="status"><Check size={18} />{copy.done}</p> : status?.waiting ? <div className="login-waiting" role="status"><span className="login-spinner" /><strong>{copy.waiting}</strong><p>{copy.waitingHint}</p></div> : <>
        <Button variant="primary" disabled={busy || !repository} onClick={() => void login()}>{copy.action}<ArrowUpRight size={16} /></Button>
        <small>{repository ? copy.browser : copy.unavailable}</small>
      </>}
      <Button disabled={busy} onClick={close}>{status?.waiting ? copy.cancel : copy.later}</Button>
      {error && <p role="alert">{error}</p>}
    </div>
  </Modal>;
}
