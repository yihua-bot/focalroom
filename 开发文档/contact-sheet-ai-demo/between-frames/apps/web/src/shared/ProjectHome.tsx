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
    [wizard, setWizard] = useState(false);
  const [invite, setInvite] = useState(
    () => new URLSearchParams(location.hash.slice(1)).get("invite") ?? "",
  );
  const reload = useCallback(async () => {
    setProjects(await request<ProjectCard[]>("/projects", session));
  }, [session]);
  useEffect(() => {
    let active = true;
    request<ProjectCard[]>("/projects", session)
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
  return (
    <main className="fr-home">
      <div className="fr-heading">
        <div>
          <p className="fr-eyebrow">PRIVATE WORKSPACE</p>
          <h1>你的共享项目</h1>
          <p>
            只有被邀请的成员可以进入。普通成员须另获项目授权；Owner/Admin
            可管理工作区项目。
          </p>
        </div>
        <button className="fr-primary" onClick={() => setWizard(!wizard)}>
          导入共享预览包
        </button>
      </div>
      {invite && (
        <div className="fr-card fr-invite">
          <div>
            <h2>你收到一个评片邀请</h2>
            <p>当前邮箱：{session.email} · 接受后获得邀请指定的权限。</p>
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
      {projects.length ? (
        <div className="fr-projects">
          {projects.map((project) => (
            <button
              className="fr-project-card"
              key={project.id}
              onClick={() => onOpen(project.id)}
            >
              <div className="fr-project-cover">
                FR<span>{project.title.slice(0, 1)}</span>
              </div>
              <h2>{project.title}</h2>
              <p>
                {project.state === "SHARED" ? "共享评片" : "预览上传中"} · v
                {project.revision}
              </p>
              <span>进入项目 →</span>
            </button>
          ))}
        </div>
      ) : (
        <div className="fr-empty">
          <h2>留一点空间，给下一组作品。</h2>
          <p>
            打开邀请链接加入评片，或从桌面导出共享预览包，在这里建立独立的共享副本。
          </p>
        </div>
      )}
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
          "请完整选择共享包内的 *.preview.jpg，照片数量须与清单一致",
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
        在云端建立独立副本。桌面原项目仍在本地；此入口不会把它自动切换为云端权威。
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
          manifest.json
          <input
            type="file"
            accept="application/json,.json"
            onChange={(e) => void load(e.target.files?.[0])}
          />
        </label>
        <label>
          2048px JPEG 预览
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
        我允许上传这些派生预览以供受邀成员评片。原图不上传，AI
        上传不在本次授权范围内。
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
