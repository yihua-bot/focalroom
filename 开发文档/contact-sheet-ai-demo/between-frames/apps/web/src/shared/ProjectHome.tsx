import { WorkspaceMembers } from "./WorkspaceMembers";
import { useCallback, useEffect, useState } from "react";
import {
  promotionSchema,
  type PromotionManifest,
} from "@betweenframes/contracts";
import { request, type Session } from "../shared-api";
import { errorMessage as message } from "../shared-api";
type ProjectCard = {
  id: string;
  title: string;
  state: string;
  revision: number;
  workspace_id: string;
  is_owner?: boolean;
};
export function ProjectHome({
  session,
  onOpen,
}: {
  session: Session;
  onOpen: (id: string) => void;
}) {
  const [projects, setProjects] = useState<ProjectCard[]>([]),
    [error, setError] = useState(""),
    [wizard, setWizard] = useState(false),
    [busy, setBusy] = useState(""),
    [confirming, setConfirming] = useState("");
  const [invite, setInvite] = useState(
    () => new URLSearchParams(location.hash.slice(1)).get("invite") ?? "",
  );
  const reload = useCallback(async () => {
    setProjects(
      await request<ProjectCard[]>("/projects?include=archived", session),
    );
  }, [session]);
  useEffect(() => {
    let active = true;
    request<ProjectCard[]>("/projects?include=archived", session)
      .then((items) => {
        if (active) setProjects(items);
      })
      .catch((error) => {
        if (active) setError(message(error));
      });
    return () => {
      active = false;
    };
  }, [session]);
  const accept = async () => {
    try {
      const result = await request<{ project_id: string | null }>(
        "/invitations/accept",
        session,
        "POST",
        { token: invite },
      );
      history.replaceState(null, "", "/shared");
      setInvite("");
      await reload();
      if (result.project_id) onOpen(result.project_id);
    } catch (error) {
      setError(message(error));
    }
  };
  const act = async (id: string, action: "archive" | "unarchive" | "delete") => {
    setBusy(id + ":" + action);
    setError("");
    setConfirming("");
    try {
      if (action === "delete")
        await request(`/projects/${id}`, session, "DELETE");
      else await request(`/projects/${id}/${action}`, session, "POST");
      await reload();
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy("");
    }
  };
  return (
    <main className="fr-home">
      <div className="fr-heading">
        <div>
          <h1>共享项目</h1>
        </div>
        <a href="/shared#explore">摄影研究</a>
        <button className="fr-primary" onClick={() => setWizard(!wizard)}>
          导入共享预览包
        </button>
      </div>
      {invite && (
        <div className="fr-card fr-invite">
          <div>
            <h2>你收到一个评片邀请</h2>
            <p>当前邮箱：{session.email}</p>
          </div>
          <button className="fr-primary" onClick={() => void accept()}>
            接受邀请
          </button>
        </div>
      )}
      {error && (
        <p role="alert" className="fr-error">
          {error}
        </p>
      )}
      {wizard && (
        <PromotionImport
          session={session}
          onDone={(id) => {
            setWizard(false);
            onOpen(id);
          }}
        />
      )}
      <WorkspaceMembers session={session} />
      {(() => {
        const active = projects.filter((p) => p.state !== "ARCHIVED");
        const archived = projects.filter((p) => p.state === "ARCHIVED");
        const card = (project: ProjectCard, isArchived: boolean) => (
          <div className="fr-project-item" key={project.id}>
            <button
              className={`fr-project-card${isArchived ? " fr-project-archived" : ""}`}
              onClick={() => onOpen(project.id)}
            >
              <div className="fr-project-cover">
                FR<span>{project.title.slice(0, 1)}</span>
              </div>
              <h2>{project.title}</h2>
              <p>
                {isArchived
                  ? "已归档"
                  : project.state === "SHARED"
                    ? "共享评片"
                    : "预览上传中"}{" "}
                · v{project.revision}
              </p>
              <span>{isArchived ? "查看归档项目 →" : "进入项目 →"}</span>
            </button>
            {project.is_owner && (
              <div className="fr-project-actions">
                {isArchived ? (
                  <>
                    <button
                      type="button"
                      disabled={busy !== ""}
                      onClick={() => void act(project.id, "unarchive")}
                    >
                      {busy === project.id + ":unarchive" ? "恢复中…" : "恢复项目"}
                    </button>
                    {confirming === project.id ? (
                      <span className="fr-project-confirm">
                        删除后不可恢复
                        <button
                          type="button"
                          className="fr-danger"
                          disabled={busy !== ""}
                          onClick={() => void act(project.id, "delete")}
                        >
                          {busy === project.id + ":delete" ? "删除中…" : "确认删除"}
                        </button>
                        <button type="button" disabled={busy !== ""} onClick={() => setConfirming("")}>
                          取消
                        </button>
                      </span>
                    ) : (
                      <button
                        type="button"
                        disabled={busy !== ""}
                        onClick={() => setConfirming(project.id)}
                      >
                        删除项目
                      </button>
                    )}
                  </>
                ) : (
                  project.state === "SHARED" && (
                    <button
                      type="button"
                      disabled={busy !== ""}
                      onClick={() => void act(project.id, "archive")}
                    >
                      {busy === project.id + ":archive" ? "归档中…" : "归档项目"}
                    </button>
                  )
                )}
              </div>
            )}
          </div>
        );
        return (
          <>
            {active.length ? (
              <div className="fr-projects">{active.map((p) => card(p, false))}</div>
            ) : (
              <div className="fr-empty">
                <h2>暂无共享项目</h2>
                <p>接受邀请或导入共享预览包。</p>
              </div>
            )}
            {archived.length > 0 && (
              <section className="fr-archived">
                <h2>已归档</h2>
                <div className="fr-projects">{archived.map((p) => card(p, true))}</div>
              </section>
            )}
          </>
        );
      })()}
    </main>
  );
}
function PromotionImport({
  session,
  onDone,
}: {
  session: Session;
  onDone: (id: string) => void;
}) {
  const [manifest, setManifest] = useState<PromotionManifest | null>(null),
    [files, setFiles] = useState<File[]>([]),
    [workspaces, setWorkspaces] = useState<{ id: string; title: string }[]>([]),
    [workspace, setWorkspace] = useState(""),
    [consent, setConsent] = useState(false),
    [progress, setProgress] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  useEffect(() => {
    request<{ id: string; title: string }[]>("/workspaces", session)
      .then((items) => {
        setWorkspaces(items);
        setWorkspace(items[0]?.id ?? "");
      })
      .catch((e) => setError(message(e)));
  }, [session]);
  const load = async (file?: File) => {
    if (!file) return;
    try {
      if (file.size > 2 * 1024 * 1024) throw new Error("清单过大");
      setManifest(promotionSchema.parse(JSON.parse(await file.text())));
    } catch (e) {
      setError(message(e));
    }
  };
  const submit = async () => {
    if (!manifest || !consent) return;
    setBusy(true);
    setError("");
    try {
      const photos = manifest.snapshot.photos;
      if (
        photos.some(
          (p) => !files.some((f) => f.name === `${p.id}.preview.jpg`),
        ) ||
        files.length !== photos.length
      )
        throw new Error(
          "请完整选择共享包中的照片预览，数量须与项目清单一致",
        );
      const value = { ...manifest, workspace_id: workspace };
      const start = await request<{
        project_id: string;
        manifest_digest: string;
        state: string;
      }>("/promotions", session, "POST", value);
      if (start.state !== "SHARED")
        for (let i = 0; i < photos.length; i++) {
          setProgress(`正在上传预览 ${i + 1} / ${photos.length}`);
          await request(
            `/projects/${start.project_id}/previews/${encodeURIComponent(photos[i].id)}`,
            session,
            "PUT",
            files.find((f) => f.name === `${photos[i].id}.preview.jpg`)!,
          );
        }
      await request(`/projects/${start.project_id}/finalize`, session, "POST", {
        manifest_digest: start.manifest_digest,
      });
      onDone(start.project_id);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="fr-card fr-import">
      <h2>导入共享预览包</h2>
      <p>
        导入后创建共享项目，桌面原项目保留。
      </p>
      <div className="fr-form-row">
        <label>
          工作区
          <select
            value={workspace}
            onChange={(e) => setWorkspace(e.target.value)}
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.title}
              </option>
            ))}
          </select>
        </label>
        <label>
          项目清单文件
          <input
            type="file"
            accept="application/json,.json"
            onChange={(e) => void load(e.target.files?.[0])}
          />
        </label>
        <label>
          照片预览文件
          <input
            type="file"
            accept="image/jpeg"
            multiple
            onChange={(e) => setFiles(Array.from(e.target.files ?? []))}
          />
        </label>
      </div>
      {manifest && (
        <p>
          {manifest.title} · {manifest.snapshot.photos.length} 张照片 · 已选择{" "}
          {files.length} 个预览
        </p>
      )}
      <label className="fr-check">
        <input
          type="checkbox"
          checked={consent}
          onChange={(e) => setConsent(e.target.checked)}
        />
        同意上传预览供受邀成员评片，不上传原图，不授权 AI 使用。
      </label>
      {error && (
        <p role="alert" className="fr-error">
          {error}
        </p>
      )}
      <button
        className="fr-primary"
        disabled={!manifest || !workspace || !consent || busy}
        onClick={() => void submit()}
      >
        {busy ? progress || "正在校验…" : "建立共享副本"}
      </button>
    </section>
  );
}
