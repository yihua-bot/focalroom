import { useEffect, useState } from "react";
import { errorMessage, request, type Session } from "../shared-api";
type Workspace = { id: string; title: string; role: string };
type Member = {
  user_id: string;
  email: string;
  display_name: string;
  role: string;
};
export function WorkspaceMembers({ session }: { session: Session }) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]),
    [selected, setSelected] = useState(""),
    [members, setMembers] = useState<Member[]>([]),
    [email, setEmail] = useState(""),
    [role, setRole] = useState("EDITOR"),
    [link, setLink] = useState(""),
    [error, setError] = useState("");
  useEffect(() => {
    request<Workspace[]>("/workspaces", session)
      .then((items) => {
        const managed = items.filter((w) =>
          ["OWNER", "ADMIN"].includes(w.role),
        );
        setWorkspaces(managed);
        setSelected(managed[0]?.id ?? "");
      })
      .catch((e) => setError(errorMessage(e)));
  }, [session]);
  useEffect(() => {
    if (!selected) return;
    let active = true;
    request<Member[]>(`/workspaces/${selected}/members`, session)
      .then((items) => {
        if (active) setMembers(items);
      })
      .catch((e) => {
        if (active) setError(errorMessage(e));
      });
    return () => {
      active = false;
    };
  }, [selected, session]);
  const invite = async () => {
    try {
      const result = await request<{ invite_url: string }>(
        `/workspaces/${selected}/invitations`,
        session,
        "POST",
        { email, role },
      );
      setLink(result.invite_url);
      setError("");
    } catch (e) {
      setError(errorMessage(e));
    }
  };
  const remove = async (id: string) => {
    try {
      await request(`/workspaces/${selected}/members/${id}`, session, "DELETE");
      setMembers(
        await request<Member[]>(`/workspaces/${selected}/members`, session),
      );
    } catch (e) {
      setError(errorMessage(e));
    }
  };
  if (!workspaces.length) return null;
  return (
    <details className="fr-card fr-workspace-settings">
      <summary>工作区成员与邀请</summary>
      <p>
        普通成员仍须单独获得项目授权。Owner/Admin
        可管理工作区项目，请仅邀请可信任的管理员。
      </p>
      <div className="fr-form-row">
        <label>
          管理工作区
          <select
            value={selected}
            onChange={(e) => {
              setSelected(e.target.value);
              setLink("");
            }}
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.title}
              </option>
            ))}
          </select>
        </label>
        <label>
          成员邮箱
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>
        <label>
          工作区角色
          <select value={role} onChange={(e) => setRole(e.target.value)}>
            <option value="EDITOR">Editor · 普通编辑</option>
            <option value="REVIEWER">Reviewer · 审稿成员</option>
            <option value="VIEWER">Viewer · 只读成员</option>
            <option value="ADMIN">Admin · 可管理所有工作区项目</option>
          </select>
        </label>
      </div>
      <button disabled={!email.includes("@")} onClick={() => void invite()}>
        生成工作区邀请
      </button>
      {link && (
        <label>
          工作区邀请链接
          <input
            value={link}
            readOnly
            onFocus={(e) => e.currentTarget.select()}
          />
        </label>
      )}
      {error && (
        <p role="alert" className="fr-error">
          {error}
        </p>
      )}
      <div className="fr-members">
        {members.map((m) => (
          <div key={m.user_id}>
            <span>
              {m.display_name} · {m.email}
            </span>
            <small>{m.role}</small>
            {m.role !== "OWNER" && (
              <button onClick={() => void remove(m.user_id)}>
                移除工作区成员
              </button>
            )}
          </div>
        ))}
      </div>
      <p className="fr-muted">
        移除成员关系不会替代项目权限撤销；独立的 ProjectGrant
        需要在对应项目中管理。
      </p>
    </details>
  );
}
