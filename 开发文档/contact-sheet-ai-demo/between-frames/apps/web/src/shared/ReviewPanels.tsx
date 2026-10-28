import { useWebI18n } from '../i18n';
import { sharedSnapshotSchema } from "@betweenframes/contracts";
import { useEffect, useState } from "react";
import {
  request,
  versionDiffSchema,
  type Session,
  type CloudSnapshot,
  type VersionDiff,
  errorMessage as message,
} from "../shared-api";
export function AccessPanel({
  state,
  session,
  refresh,
  setError,
}: {
  state: CloudSnapshot;
  session: Session;
  refresh: () => Promise<void>;
  setError: (v: string) => void;
}) {
  const [email, setEmail] = useState(""),
    [role, setRole] = useState("REVIEWER"),
    [link, setLink] = useState(""),
    [inviteId, setInviteId] = useState("");
  const invite = async () => {
    try {
      const result = await request<{ invite_url: string; id: string }>(
        `/workspaces/${state.workspace_id}/invitations`,
        session,
        "POST",
        { email, role, project_id: state.project_id },
      );
      setLink(result.invite_url);
      setInviteId(result.id);
    } catch (e) {
      setError(message(e));
    }
  };
  const revoke = async (user: string) => {
    try {
      await request(
        `/projects/${state.project_id}/grants/${user}`,
        session,
        "DELETE",
      );
      await refresh();
    } catch (e) {
      setError(message(e));
    }
  };
  return (
    <div className="fr-panel">
      <h2>邀请成员</h2>
      <p>
        仅开放本项目。链接 7 天有效，须用指定邮箱接受一次；请自行分享邀请链接。
      </p>
      <label>
        受邀邮箱
        <input
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
      </label>
      <label>
        项目角色
        <select value={role} onChange={(e) => setRole(e.target.value)}>
          <option value="REVIEWER">Reviewer · 评论与提案</option>
          <option value="VIEWER">Viewer · 只读预览</option>
          <option value="EDITOR">Editor · 编辑（审批须单独授权）</option>
        </select>
      </label>
      <button
        className="fr-primary"
        disabled={!email.includes("@")}
        onClick={() => void invite()}
      >
        生成私密邀请
      </button>
      {link && (
        <div className="fr-invite-link">
          <label>
            邀请链接
            <input
              readOnly
              value={link}
              onFocus={(e) => e.currentTarget.select()}
            />
          </label>
          <button
            onClick={() =>
              void navigator.clipboard
                .writeText(link)
                .catch((e) => setError(message(e)))
            }
          >
            复制链接
          </button>
          <button
            onClick={() =>
              void request(
                `/workspaces/${state.workspace_id}/invitations/${inviteId}`,
                session,
                "DELETE",
              )
                .then(() => {
                  setLink("");
                  setInviteId("");
                })
                .catch((e) => setError(message(e)))
            }
          >
            撤销此邀请
          </button>
        </div>
      )}
      <h3>项目访问权限</h3>
      {state.grants.map((g) => (
        <article className="fr-grant" key={g.user_id}>
          <strong>{g.email}</strong>
          <p>
            {g.role} · {g.active ? "访问中" : "已撤销"}
          </p>
          {g.active && (
            <button onClick={() => void revoke(g.user_id)}>撤销访问</button>
          )}
          {g.active && g.role === "EDITOR" && (
            <label className="fr-check">
              <input
                type="checkbox"
                checked={g.can_approve}
                onChange={(e) =>
                  void request(
                    `/projects/${state.project_id}/grants/${g.user_id}`,
                    session,
                    "PATCH",
                    {
                      role: "EDITOR",
                      can_approve: e.target.checked,
                      can_edit_formal: g.can_edit_formal,
                    },
                  )
                    .then(refresh)
                    .catch((e) => setError(message(e)))
                }
              />
              允许审批提案
            </label>
          )}
        </article>
      ))}
      <p className="fr-muted">
        撤销会阻止后续预览请求并关闭实时连接。已经下载的照片像素无法收回。
      </p>
    </div>
  );
}
export function HistoryPanel({
  id,
  session,
  revision,
  photos,
}: {
  id: string;
  session: Session;
  revision: number;
  photos: CloudSnapshot["snapshot"]["photos"];
}) {
  const { fmtDate } = useWebI18n();
  const [viewing, setViewing] = useState<{
    revision: number;
    snapshot: CloudSnapshot["snapshot"];
  } | null>(null);
  const [history, setHistory] = useState<
      { revision: number; created_at: string; command_id: string }[]
    >([]),
    [error, setError] = useState("");
  const [diffPair, setDiffPair] = useState({ from: "", to: "" });
  const [versionDiff, setVersionDiff] = useState<VersionDiff | "loading" | null>(
    null,
  );
  const filenameOf = (photoId: string) =>
    photos.find((photo) => photo.id === photoId)?.filename ?? photoId;
  const loadVersionDiff = async () => {
    setVersionDiff("loading");
    try {
      const diff = versionDiffSchema.parse(
        await request(
          `/projects/${id}/versions/${diffPair.from}/diff/${diffPair.to}`,
          session,
        ),
      );
      setVersionDiff(diff);
    } catch (e) {
      setError(message(e));
      setVersionDiff(null);
    }
  };
  useEffect(() => {
    request<typeof history>(`/projects/${id}/history`, session)
      .then(setHistory)
      .catch((e) => setError(message(e)));
  }, [id, session, revision]);
  return (
    <div className="fr-panel">
      <h2>版本历史</h2>
      {error && <p role="alert">{error}</p>}
      {viewing && (
        <section className="fr-card">
          <strong>正在查看 v{viewing.revision} · 只读</strong>
          <ol>
            {viewing.snapshot.sequences[0].items.map((item) => (
              <li key={item.id}>
                {
                  viewing.snapshot.photos.find((p) => p.id === item.photo_id)
                    ?.filename
                }
              </li>
            ))}
          </ol>
        </section>
      )}
      {history.length > 1 && (
        <section className="fr-version-diff">
          <h3>版本对比</h3>
          <label>
            从
            <select
              aria-label="对比起始版本"
              value={diffPair.from}
              onChange={(e) =>
                setDiffPair((prev) => ({ ...prev, from: e.target.value }))
              }
            >
              <option value="">选择版本</option>
              {history.map((v) => (
                <option key={v.revision} value={String(v.revision)}>
                  v{v.revision}
                </option>
              ))}
            </select>
          </label>
          <label>
            到
            <select
              aria-label="对比目标版本"
              value={diffPair.to}
              onChange={(e) =>
                setDiffPair((prev) => ({ ...prev, to: e.target.value }))
              }
            >
              <option value="">选择版本</option>
              {history.map((v) => (
                <option key={v.revision} value={String(v.revision)}>
                  v{v.revision}
                </option>
              ))}
            </select>
          </label>
          <button
            disabled={
              !diffPair.from || !diffPair.to || diffPair.from === diffPair.to
            }
            onClick={() => void loadVersionDiff()}
          >
            对比差异
          </button>
          {versionDiff === "loading" && <p>差异计算中…</p>}
          {versionDiff && versionDiff !== "loading" && (
            <div className="fr-diff">
              <p>
                v{versionDiff.from.revision} → v{versionDiff.to.revision}
              </p>
              {versionDiff.photos.added.length > 0 && (
                <p>
                  新增照片：
                  {versionDiff.photos.added.map(filenameOf).join("、")}
                </p>
              )}
              {versionDiff.photos.removed.length > 0 && (
                <p>
                  移除照片：
                  {versionDiff.photos.removed.map(filenameOf).join("、")}
                </p>
              )}
              {versionDiff.photos.changed.map((entry) => (
                <p key={entry.id}>
                  {filenameOf(entry.id)}：
                  {Object.entries(entry.fields)
                    .map(
                      ([field, change]) =>
                        `${field} ${String(change.before)} → ${String(change.after)}`,
                    )
                    .join("；")}
                </p>
              ))}
              {(versionDiff.sequences.added.length > 0 ||
                versionDiff.sequences.removed.length > 0 ||
                versionDiff.sequences.changed.length > 0) && (
                <p>
                  序列：新增 {versionDiff.sequences.added.length} · 移除{" "}
                  {versionDiff.sequences.removed.length} · 变更{" "}
                  {versionDiff.sequences.changed.length}
                </p>
              )}
              {versionDiff.photos.added.length === 0 &&
                versionDiff.photos.removed.length === 0 &&
                versionDiff.photos.changed.length === 0 &&
                versionDiff.sequences.added.length === 0 &&
                versionDiff.sequences.removed.length === 0 &&
                versionDiff.sequences.changed.length === 0 && (
                  <p>两个版本内容一致。</p>
                )}
            </div>
          )}
        </section>
      )}
      {history.map((v) => (
        <article className="fr-version" key={v.revision}>
          <strong>v{v.revision}</strong>
          <button
            onClick={() =>
              void request<{ revision: number; snapshot: unknown }>(
                "/projects/" + id + "/versions/" + v.revision,
                session,
              )
                .then((result) =>
                  setViewing({
                    revision: result.revision,
                    snapshot: sharedSnapshotSchema.parse(result.snapshot),
                  }),
                )
                .catch((e) => setError(message(e)))
            }
          >
            查看版本
          </button>
          <time>{fmtDate(v.created_at)}</time>
          <small>
            {v.command_id.startsWith("promotion:")
              ? "共享副本建立"
              : "作品更新"}
          </small>
        </article>
      ))}
    </div>
  );
}
