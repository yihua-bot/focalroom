import { useWebI18n } from '../i18n';
import { sharedSnapshotSchema } from "@betweenframes/contracts";
import { useEffect, useState } from "react";
import {
  request,
  type Session,
  type CloudSnapshot,
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
}: {
  id: string;
  session: Session;
  revision: number;
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
