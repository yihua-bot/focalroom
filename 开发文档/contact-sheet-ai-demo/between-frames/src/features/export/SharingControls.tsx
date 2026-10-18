import { CloudAccount, useCloudStatus } from "../cloud-login/CloudLoginView";
import { useEffect, useState } from "react";
import type { SharingRepository, SharingStatus } from "../../domain/sharing";
import { Button } from "../../components/shared/Button";
import { SharedReviewLink } from "../../components/shared/SharedReviewLink";
export function SharingControls({
  repository,
  projectId,
  revision,
  onChanged,
  onBusyChange,
}: {
  repository: SharingRepository;
  projectId: string;
  revision: number;
  onChanged?: () => void;
  onBusyChange?: (busy: boolean) => void;
}) {
  const [status, setStatus] = useState<SharingStatus | null>(null),
    [baseInput, setBase] = useState<string|null>(null),
    [webInput, setWeb] = useState<string|null>(null),
    [consent, setConsent] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const {status: account} = useCloudStatus(repository.auth);
  const defaultOrigin = account?.authenticated ? account.origin : "";
  const base = status?.remote?.binding?.baseUrl ?? baseInput ?? defaultOrigin;
  const web = status?.remote?.binding?.webUrl ?? webInput ?? defaultOrigin;
  useEffect(() => {
    let active = true;
    repository
      .status(projectId)
      .then((result) => {
        if (active) {
          setStatus(result);
        }
      })
      .catch(() => {
        if (active) setError("共享状态读取失败，请重新打开窗口");
      });
    return () => {
      active = false;
    };
  }, [repository, projectId]);
  const run = async (action: "resume" | "abort") => {
    setBusy(true);
    onBusyChange?.(true);
    setError("");
    try {
      setStatus(
        await repository.share({
          projectId,
          expectedRevision: revision,
          baseUrl: base,
          webUrl: web,
          action,
          previewUploadAuthorized: consent,
        }),
      );
    } catch (error) {
      setError(error instanceof Error ? error.message : "共享失败");
    } finally {
      setBusy(false);
      onBusyChange?.(false);
      const current = await repository.status(projectId).catch(() => null);
      if (current) setStatus(current);
      onChanged?.();
    }
  };
  return (
    <section className="info-card sharing-controls">
      <h3>共享项目</h3>
      {repository.auth && <CloudAccount repository={repository.auth} />}
      {account?.authenticated && <p>{account.user?.email}</p>}
      <p>
        迁移期间暂停编辑；完成后请在 Web 继续评片，桌面只读。原图和个人记忆留在本机。
      </p>
      {status?.authority === "shared" ? (
        <p role="status">
          已迁移至共享评片。
          {status.remote?.webUrl && (
            <SharedReviewLink projectId={projectId} url={status.remote.webUrl} />
          )}
        </p>
      ) : (
        <>
          <p>
            状态：
            {status?.authority === "local-promoting"
              ? "迁移未完成，请使用原服务和账户继续。"
              : "本地项目"}
          </p>
          <div className="sharing-fields">
            <label>
              API 服务地址
              <input
                type="url"
                value={base}
                placeholder="https://api.example.com"
                disabled={busy || !!status?.remote?.binding}
                onChange={(e) => setBase(e.target.value)}
              />
            </label>
            <label>
              Web 评片地址
              <input
                type="url"
                value={web}
                placeholder="https://app.example.com"
                disabled={busy || !!status?.remote?.binding}
                onChange={(e) => setWeb(e.target.value)}
              />
            </label>
          </div>
          <label className="sync-toggle">
            <input
              type="checkbox"
              checked={consent}
              disabled={busy}
              onChange={(e) => setConsent(e.target.checked)}
            />
            我授权向上述共享服务上传去元数据的派生预览，并了解完成后本地正式编辑只读。此授权不包含
            AI 上传。
          </label>
          <div className="dialog-actions">
            <Button
              disabled={
                busy ||
                !base ||
                !web ||
                !account?.authenticated ||
                base !== account.origin ||
                !consent ||
                !status
              }
              variant="primary"
              onClick={() => void run("resume")}
            >
              {busy
                ? "正在共享，请保持窗口打开…"
                : status?.authority === "local-promoting"
                  ? "恢复共享迁移"
                  : "确认迁移为共享项目"}
            </Button>
            {status?.authority === "local-promoting" && (
              <Button
                disabled={busy || !account?.authenticated || base !== account.origin || !consent}
                onClick={() => void run("abort")}
              >
                验证云端未提交并取消迁移
              </Button>
            )}
          </div>
        </>
      )}
      {error && (
        <p role="alert" className="error-message">
          {error}
        </p>
      )}
    </section>
  );
}
