import { useCallback, useEffect, useState } from "react";
import {
  CloudError,
  commandFor,
  request,
  snapshotSchema,
  type CloudSnapshot,
  type DraftOperation,
  type Session,
  errorMessage as message,
} from "../shared-api";
import { useRealtime } from "./useRealtime";
import { AccessPanel, HistoryPanel } from "./ReviewPanels";

type CommentTarget =
  | { kind: "photo" }
  | { kind: "sequence"; itemId: string; index: number }
  | { kind: "revision" };

export function ProjectReview({
  id,
  session,
  onBack,
}: {
  id: string;
  session: Session;
  onBack: () => void;
}) {
  const [state, setState] = useState<CloudSnapshot | null>(null),
    [error, setError] = useState(""),
    [revoked, setRevoked] = useState(false),
    [selected, setSelected] = useState(""),
    [images, setImages] = useState<Record<string, string>>({}),
    [tab, setTab] = useState<"comments" | "proposals" | "access" | "history">(
      "comments",
    );
  const [body, setBody] = useState(""),
    [anchor, setAnchor] = useState({ x: 0.5, y: 0.5 }),
    [commentTarget, setCommentTarget] = useState<CommentTarget>({
      kind: "photo",
    }),
    [replyTo, setReply] = useState<string | null>(null),
    [operations, setOperations] = useState<DraftOperation[]>([]),
    [title, setTitle] = useState(""),
    [accepted, setAccepted] = useState<Record<string, string[]>>({}),
    [busy, setBusy] = useState(false);
  const [editingProposal, setEditingProposal] = useState<{
    id: string;
    revision: number;
  } | null>(null);
  const [photoPage, setPhotoPage] = useState(0),
    [sequencePage, setSequencePage] = useState(0);
  const [presenceDetail, setPresenceDetail] = useState(() => {
    try {
      return window.localStorage.getItem("focalroom.presence.detail") !== "off";
    } catch {
      return true;
    }
  });
  const visiblePhotos =
    state?.snapshot.photos.slice(photoPage * 48, (photoPage + 1) * 48) ?? [];
  const visibleSequence =
    state?.snapshot.sequences[0].items.slice(
      sequencePage * 48,
      (sequencePage + 1) * 48,
    ) ?? [];
  const revoke = useCallback(() => {
    setRevoked(true);
    setState(null);
    setImages({});
    setError(
      "访问已撤销或角色已变化，请返回项目列表。已下载的像素无法从设备收回。",
    );
  }, []);
  const refresh = useCallback(async () => {
    try {
      const result = snapshotSchema.parse(
        await request(`/projects/${id}/snapshot`, session),
      );
      setState(result);
      setSelected((current) => current || result.snapshot.photos[0]?.id || "");
    } catch (e) {
      if (
        e instanceof CloudError &&
        ["PERMISSION_DENIED", "AUTH_REVOKED"].includes(e.code)
      )
        revoke();
      else setError(message(e));
    }
  }, [id, session, revoke]);
  useEffect(() => {
    let active = true;
    request(`/projects/${id}/snapshot`, session)
      .then((value) => {
        if (active) {
          const next = snapshotSchema.parse(value);
          setState(next);
          setSelected(next.snapshot.photos[0]?.id ?? "");
        }
      })
      .catch((e) => {
        if (active) setError(message(e));
      });
    return () => {
      active = false;
    };
  }, [id, session]);
  const realtime = useRealtime(id, session, refresh, revoke);
  const visibleIds = new Set([
    ...visiblePhotos.map((p) => p.id),
    ...visibleSequence.map((p) => p.photo_id),
    selected,
  ]);
  const previewKey = JSON.stringify(
    state?.previews
      .filter((p) => visibleIds.has(p.photo_id))
      .map((p) => [p.photo_id, p.content_sha256]) ?? [],
  );
  useEffect(() => {
    let active = true;
    const urls: string[] = [];
    const load = async () => {
      const result: Record<string, string> = {};
      for (const [photoId] of JSON.parse(previewKey) as [string, string][]) {
        if (!active) return;
        const ticket = await request<{ url: string }>(
          `/projects/${id}/previews/${encodeURIComponent(photoId)}/ticket`,
          session,
          "POST",
        );
        const response = await fetch(ticket.url, {
          credentials: "same-origin",
        });
        if (!response.ok) throw new Error("预览访问失败，请刷新项目");
        const blob = await response.blob();
        if (!active) return;
        const url = URL.createObjectURL(blob);
        urls.push(url);
        result[photoId] = url;
      }
      if (active) setImages(result);
    };
    if (previewKey)
      void load().catch((e) => {
        if (active) setError(message(e));
      });
    return () => {
      active = false;
      for (const url of urls) URL.revokeObjectURL(url);
    };
  }, [id, session, previewKey]);
  const mutate = async (kind: string, payload: unknown) => {
    if (!state || !realtime.online) return;
    setBusy(true);
    setError("");
    try {
      await request(
        `/projects/${id}/commands`,
        session,
        "POST",
        commandFor(state, kind, payload),
      );
      await refresh();
      return true;
    } catch (e) {
      setError(message(e));
      await refresh();
      return false;
    } finally {
      setBusy(false);
    }
  };
  const addComment = async () => {
    if (!state || !body.trim()) return;
    if (replyTo) {
      try {
        await request(
          `/projects/${id}/comments/${replyTo}/replies`,
          session,
          "POST",
          { body, idempotency_key: crypto.randomUUID() },
        );
        setBody("");
        setReply(null);
        await refresh();
      } catch (e) {
        setError(message(e));
      }
      return;
    }
    const target =
      commentTarget.kind === "sequence"
        ? {
            anchor: {
              type: "sequence" as const,
              sequence_item_id: commentTarget.itemId,
              revision_id: String(state.revision),
            },
          }
        : commentTarget.kind === "revision"
          ? {
              anchor: {
                type: "revision" as const,
                revision_id: String(state.revision),
              },
            }
          : {
              anchor: {
                type: "photo" as const,
                photo_id: selected,
                ...anchor,
              },
            };
    if (await mutate("comment.add", { body, ...target })) {
      setBody("");
      setCommentTarget({ kind: "photo" });
    }
  };
  const saveProposal = async (submit: boolean) => {
    if (!state || !realtime.online || !title.trim() || !operations.length)
      return;
    setBusy(true);
    setError("");
    const cmd = commandFor(state, "proposal.create", {
      title,
      base_revision: state.revision,
      operations,
    });
    let proposalId: string = cmd.command_id,
      proposalRevision = 0;
    try {
      if (editingProposal) {
        const reply = await request<{ proposal_revision: number }>(
          "/projects/" + id + "/proposals/" + editingProposal.id,
          session,
          "PATCH",
          {
            idempotency_key: crypto.randomUUID(),
            expected_proposal_revision: editingProposal.revision,
            base_revision: state.revision,
            title,
            operations,
          },
        );
        proposalId = editingProposal.id;
        proposalRevision = reply.proposal_revision;
      } else
        await request("/projects/" + id + "/commands", session, "POST", cmd);
      if (submit)
        await request(
          "/projects/" + id + "/commands",
          session,
          "POST",
          commandFor(state, "proposal.submit", {
            proposal_id: proposalId,
            expected_proposal_revision: proposalRevision,
          }),
        );
      setOperations([]);
      setTitle("");
      setEditingProposal(null);
      await refresh();
    } catch (e) {
      setError(message(e));
      await refresh();
    } finally {
      setBusy(false);
    }
  };
  if (!state)
    return (
      <main className="fr-home">
        <button onClick={onBack}>← 返回项目</button>
        {error ? (
          <p role="alert" className="fr-error">
            {error}
          </p>
        ) : (
          <p>正在读取项目…</p>
        )}
        {revoked && <p>本页已清空项目内容。</p>}
      </main>
    );
  const photo = state.snapshot.photos.find((p) => p.id === selected),
    canWrite = state.principal.role !== "VIEWER" && realtime.online && !busy,
    canApprove =
      ["OWNER", "ADMIN"].includes(state.principal.role) ||
      state.principal.can_approve;
  const photoComments = state.comments.filter(
    (c) => c.anchor.photo_id === selected && !c.parent_id,
  );
  const visiblePeople = realtime.people.filter(
    (p, i, all) => all.findIndex((v) => v.id === p.id) === i,
  );
  const threadComments = state.comments.filter(
    (c) =>
      !c.parent_id &&
      (c.anchor.type === "sequence" || c.anchor.type === "revision"),
  );
  const anchorLabel = (c: CloudSnapshot["comments"][number]) =>
    c.anchor.type === "revision"
      ? `版本 v${c.anchor.revision_id}`
      : `序列条目 · v${c.anchor.revision_id}`;
  const renderThread = (c: CloudSnapshot["comments"][number]) => (
    <article className="fr-comment" key={c.id}>
      <strong>{c.reviewer}</strong>
      <small>
        {c.anchor.type !== "photo"
          ? `${anchorLabel(c)} · `
          : ""}
        {c.state === "RESOLVED" ? "已解决" : "待处理"}
      </small>
      <p>{c.body}</p>
      {state.comments
        .filter((r) => r.parent_id === c.id)
        .map((r) => (
          <div className="fr-reply" key={r.id}>
            <strong>{r.reviewer}</strong>
            <p>{r.body}</p>
          </div>
        ))}
      <div className="fr-inline">
        <button disabled={!canWrite} onClick={() => setReply(c.id)}>
          回复
        </button>
        <button
          disabled={
            !canWrite ||
            (c.author_id !== session.id &&
              !["OWNER", "ADMIN", "EDITOR"].includes(state.principal.role))
          }
          onClick={() =>
            void mutate("comment.resolve", {
              thread_id: c.id,
              state: c.state === "RESOLVED" ? "REOPENED" : "RESOLVED",
            })
          }
        >
          {c.state === "RESOLVED" ? "重新打开" : "标记解决"}
        </button>
      </div>
    </article>
  );
  const proposalSelection = (p: CloudSnapshot["proposals"][number]) =>
    accepted[p.id] ?? p.operations.map((o) => o.operation_id);
  const choose = (photoId: string) => {
    setSelected(photoId);
    setReply(null);
    realtime.viewPhoto(photoId);
  };
  return (
    <>
      <div className="fr-project-bar">
        <button onClick={onBack}>← 项目</button>
        <div>
          <h1>{state.title}</h1>
          <span>
            v{state.revision} · {state.principal.role}
          </span>
        </div>
        <div className="fr-presence">
          <span className={realtime.online ? "fr-online" : "fr-offline"}>
            {realtime.online ? "实时连接" : "离线只读"}
          </span>
          <button
            className="fr-presence-toggle"
            aria-pressed={presenceDetail}
            aria-label={presenceDetail ? "隐藏协作者动态" : "显示协作者动态"}
            onClick={() => {
              setPresenceDetail((value) => {
                try {
                  window.localStorage.setItem(
                    "focalroom.presence.detail",
                    value ? "off" : "on",
                  );
                } catch {
                  /* 隐私模式忽略持久化。 */
                }
                return !value;
              });
            }}
          >
            {visiblePeople.length} 人在线
          </button>
          {presenceDetail &&
            visiblePeople.map((p) => {
              const viewing = p.photo_id
                ? (state.snapshot.photos.find(
                    (photo) => photo.id === p.photo_id,
                  )?.filename ?? null)
                : null;
              return (
                <span
                  className="fr-person"
                  key={p.id}
                  title={
                    viewing ? `${p.name} · 正在查看 ${viewing}` : p.name
                  }
                >
                  {p.name.slice(0, 1).toUpperCase()}
                  {viewing && <small className="fr-person-viewing">{viewing}</small>}
                </span>
              );
            })}
        </div>
      </div>
      {error && (
        <div role="alert" className="fr-error">
          {error}
        </div>
      )}
      {!realtime.online && (
        <div className="fr-notice">
          连接已断开，暂时无法提交。未提交的提案保留在本页，重连后请核对最新版本。
        </div>
      )}
      <main className="fr-review">
        <aside className="fr-assets">
          <h2>
            项目照片 <span>{state.snapshot.photos.length}</span>
          </h2>
          {state.snapshot.photos.length > 48 && (
            <div className="fr-pagination">
              <button
                aria-label="上一页照片"
                disabled={photoPage === 0}
                onClick={() => setPhotoPage((p) => p - 1)}
              >
                ←
              </button>
              <span>
                {photoPage + 1} / {Math.ceil(state.snapshot.photos.length / 48)}
              </span>
              <button
                aria-label="下一页照片"
                disabled={(photoPage + 1) * 48 >= state.snapshot.photos.length}
                onClick={() => setPhotoPage((p) => p + 1)}
              >
                →
              </button>
            </div>
          )}
          <div className="fr-photo-grid">
            {visiblePhotos.map((p, i) => (
              <button
                key={p.id}
                aria-label={`查看 ${p.filename}`}
                aria-pressed={selected === p.id}
                onClick={() => choose(p.id)}
              >
                {images[p.id] ? (
                  <img src={images[p.id]} alt={p.filename} />
                ) : (
                  <div className="fr-image-placeholder" />
                )}
                <span>{String(photoPage * 48 + i + 1).padStart(2, "0")}</span>
                <small>
                  {p.selection === "KEEP"
                    ? "保留"
                    : p.selection === "REJECT"
                      ? "排除"
                      : "待选"}
                </small>
              </button>
            ))}
          </div>
        </aside>
        <section className="fr-stage">
          <div className="fr-stage-head">
            <span>{photo?.filename}</span>
            <span>
              {commentTarget.kind === "photo"
                ? "点击预览放置批注锚点"
                : "正在批注序列或版本；发布后自动返回照片批注"}
            </span>
          </div>
          <div className="fr-inspector">
            {photo && images[selected] && (
              <div
                className="fr-image-frame"
                onClick={(event) => {
                  const box = event.currentTarget.getBoundingClientRect();
                  setAnchor({
                    x: Math.max(
                      0,
                      Math.min(1, (event.clientX - box.left) / box.width),
                    ),
                    y: Math.max(
                      0,
                      Math.min(1, (event.clientY - box.top) / box.height),
                    ),
                  });
                }}
              >
                <img src={images[selected]} alt={`检查 ${photo.filename}`} />
                <span
                  className="fr-anchor"
                  style={{
                    left: `${anchor.x * 100}%`,
                    top: `${anchor.y * 100}%`,
                  }}
                >
                  +
                </span>
                {state.comments
                  .filter(
                    (c) =>
                      c.anchor.photo_id === selected &&
                      !c.parent_id &&
                      c.state !== "RESOLVED",
                  )
                  .map((c, i) => (
                    <button
                      aria-label={`批注 ${i + 1}`}
                      className="fr-comment-pin"
                      style={{
                        left: `${(c.anchor.x ?? 0) * 100}%`,
                        top: `${(c.anchor.y ?? 0) * 100}%`,
                      }}
                      key={c.id}
                      onClick={(e) => {
                        e.stopPropagation();
                        setTab("comments");
                        setReply(c.id);
                      }}
                    >
                      {i + 1}
                    </button>
                  ))}
              </div>
            )}
          </div>
          <div className="fr-sequence">
            <h2>
              正式作品序列{" "}
              <span>
                {state.snapshot.sequences[0].items.length} 张 · v
                {state.revision}
              </span>
            </h2>
            {state.snapshot.sequences[0].items.length > 48 && (
              <div className="fr-pagination">
                <button
                  aria-label="上一页序列"
                  disabled={sequencePage === 0}
                  onClick={() => setSequencePage((p) => p - 1)}
                >
                  ←
                </button>
                <span>
                  {sequencePage + 1} /{" "}
                  {Math.ceil(state.snapshot.sequences[0].items.length / 48)}
                </span>
                <button
                  aria-label="下一页序列"
                  disabled={
                    (sequencePage + 1) * 48 >=
                    state.snapshot.sequences[0].items.length
                  }
                  onClick={() => setSequencePage((p) => p + 1)}
                >
                  →
                </button>
              </div>
            )}
            <div className="fr-sequence-strip">
              {visibleSequence.map((item, i) => {
                const notes = state.comments.filter(
                  (c) =>
                    !c.parent_id &&
                    c.anchor.sequence_item_id === item.id &&
                    c.state !== "RESOLVED",
                ).length;
                return (
                  <div className="fr-seq-item" key={item.id}>
                    <button
                      aria-label={`序列 ${sequencePage * 48 + i + 1}`}
                      onClick={() => choose(item.photo_id)}
                    >
                      {images[item.photo_id] && (
                        <img src={images[item.photo_id]} alt="" />
                      )}
                      <span>{i + 1}</span>
                      {notes > 0 && (
                        <small
                          className="fr-seq-count"
                          aria-label={`${notes} 条序列批注`}
                        >
                          {notes}
                        </small>
                      )}
                    </button>
                    <button
                      className="fr-seq-note"
                      aria-label={`批注序列第 ${sequencePage * 48 + i + 1} 张`}
                      disabled={state.principal.role === "VIEWER"}
                      onClick={() => {
                        setCommentTarget({
                          kind: "sequence",
                          itemId: item.id,
                          index: sequencePage * 48 + i,
                        });
                        setTab("comments");
                      }}
                    >
                      批注
                    </button>
                  </div>
                );
              })}
              {!state.snapshot.sequences[0].items.length && (
                <p>尚未编排。通过提案提交第一组序列。</p>
              )}
            </div>
          </div>
        </section>
        <aside className="fr-discussion">
          <nav aria-label="评片侧栏">
            <button
              aria-pressed={tab === "comments"}
              onClick={() => setTab("comments")}
            >
              批注
            </button>
            <button
              aria-pressed={tab === "proposals"}
              onClick={() => setTab("proposals")}
            >
              提案
            </button>
            <button
              aria-pressed={tab === "history"}
              onClick={() => setTab("history")}
            >
              版本
            </button>
            {["OWNER", "ADMIN"].includes(state.principal.role) && (
              <button
                aria-pressed={tab === "access"}
                onClick={() => setTab("access")}
              >
                权限
              </button>
            )}
          </nav>
          {tab === "comments" && (
            <div className="fr-panel">
              <h2>批注</h2>
              <p className="fr-muted">
                {commentTarget.kind === "photo" ? (
                  <>
                    {photo?.filename} · {photoComments.length} 条批注
                  </>
                ) : commentTarget.kind === "sequence" ? (
                  <>
                    目标：序列第 {commentTarget.index + 1} 张 · v
                    {state.revision}
                  </>
                ) : (
                  <>目标：作品版本 v{state.revision}</>
                )}
                {commentTarget.kind !== "photo" && (
                  <button onClick={() => setCommentTarget({ kind: "photo" })}>
                    改回照片批注
                  </button>
                )}
              </p>
              <div className="fr-inline">
                <button
                  disabled={!canWrite}
                  onClick={() => setCommentTarget({ kind: "revision" })}
                >
                  批注当前版本 v{state.revision}
                </button>
              </div>
              {photoComments.map(renderThread)}
              {threadComments.length > 0 && (
                <>
                  <h3 className="fr-anchor-heading">序列与版本批注</h3>
                  {threadComments.map(renderThread)}
                </>
              )}
              {replyTo && (
                <p>
                  正在回复批注{" "}
                  <button onClick={() => setReply(null)}>取消回复</button>
                </p>
              )}
              <label>
                {replyTo
                  ? "回复内容"
                  : commentTarget.kind === "sequence"
                    ? "序列批注"
                    : commentTarget.kind === "revision"
                      ? "版本批注"
                      : "添加批注"}
                <textarea
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  maxLength={5000}
                  placeholder="描述你的观察，或给出具体的编辑建议…"
                  disabled={state.principal.role === "VIEWER"}
                />
              </label>
              <button
                className="fr-primary"
                disabled={!canWrite || !body.trim()}
                onClick={() => void addComment()}
              >
                {replyTo ? "发布回复" : "发布批注"}
              </button>
              <label className="fr-note-label">
                私人笔记
                <textarea
                  aria-label="私人笔记"
                  value={realtime.note}
                  onChange={(e) => realtime.updateNote(e.target.value)}
                  maxLength={4000}
                  disabled={!canWrite}
                />
              </label>
            </div>
          )}
          {tab === "proposals" && (
            <div className="fr-panel">
              <h2>编辑提案</h2>
              {state.proposals.map((p) => (
                <article className="fr-proposal" key={p.id}>
                  <h3>{p.title}</h3>
                  <p>
                    {p.author} · 基于 v{p.base_revision} · {p.state}
                  </p>
                  {p.operations.map((op) => (
                    <label className="fr-check" key={op.operation_id}>
                      <input
                        type="checkbox"
                        checked={proposalSelection(p).includes(op.operation_id)}
                        disabled={!canApprove || p.state !== "SUBMITTED"}
                        onChange={(e) =>
                          setAccepted((prev) => ({
                            ...prev,
                            [p.id]: e.target.checked
                              ? [...proposalSelection(p), op.operation_id]
                              : proposalSelection(p).filter(
                                  (id) => id !== op.operation_id,
                                ),
                          }))
                        }
                      />
                      <span>
                        {op.op === "add"
                          ? "新增"
                          : op.op === "remove"
                            ? "移除"
                            : "重排"}{" "}
                        ·{" "}
                        {
                          state.snapshot.photos.find(
                            (photo) => photo.id === op.photo_id,
                          )?.filename
                        }{" "}
                        {op.to_index !== undefined
                          ? `→ 第 ${op.to_index + 1} 张`
                          : ""}
                      </span>
                    </label>
                  ))}
                  {p.state === "DRAFT" && p.author_id === session.id && (
                    <button
                      disabled={!canWrite}
                      onClick={() => {
                        setEditingProposal({ id: p.id, revision: p.revision });
                        setTitle(p.title);
                        setOperations(p.operations);
                      }}
                    >
                      继续编辑草案
                    </button>
                  )}
                  {p.state === "SUBMITTED" &&
                    p.base_revision !== state.revision && (
                      <button
                        disabled={!canWrite}
                        onClick={() => {
                          setEditingProposal(null);
                          setTitle(p.title + " · 当前版本");
                          setOperations(
                            p.operations.map((op) => ({
                              ...op,
                              operation_id: crypto.randomUUID(),
                            })),
                          );
                        }}
                      >
                        以当前版本重建提案
                      </button>
                    )}
                  {p.state === "DRAFT" && p.author_id === session.id && (
                    <button
                      disabled={!canWrite}
                      onClick={() =>
                        void mutate("proposal.submit", {
                          proposal_id: p.id,
                          expected_proposal_revision: p.revision,
                        })
                      }
                    >
                      提交评审
                    </button>
                  )}
                  {p.state === "SUBMITTED" && canApprove && (
                    <>
                      <button
                        className="fr-primary"
                        disabled={
                          !realtime.online ||
                          busy ||
                          p.base_revision !== state.revision
                        }
                        onClick={() =>
                          void mutate("proposal.apply", {
                            proposal_id: p.id,
                            expected_proposal_revision: p.revision,
                            accepted_operation_ids: proposalSelection(p),
                            approval_nonce: p.nonce,
                          })
                        }
                      >
                        {proposalSelection(p).length
                          ? "批准选中改动"
                          : "驳回提案"}
                      </button>
                      <button
                        disabled={
                          !realtime.online ||
                          busy ||
                          p.base_revision !== state.revision
                        }
                        onClick={() =>
                          void mutate("proposal.apply", {
                            proposal_id: p.id,
                            expected_proposal_revision: p.revision,
                            accepted_operation_ids: [],
                            approval_nonce: p.nonce,
                          })
                        }
                      >
                        全部驳回
                      </button>
                    </>
                  )}
                  {p.base_revision !== state.revision &&
                    p.state === "SUBMITTED" && (
                      <p className="fr-warning">
                        作品已更新，原提案已保留。请按最新版本重建。
                      </p>
                    )}
                </article>
              ))}
              {state.principal.role !== "VIEWER" && (
                <section className="fr-proposal-builder">
                  <h3>{editingProposal ? "修订提案草案" : "新建序列提案"}</h3>
                  {editingProposal && (
                    <button
                      onClick={() => {
                        setEditingProposal(null);
                        setOperations([]);
                        setTitle("");
                      }}
                    >
                      取消本次修订
                    </button>
                  )}
                  <label>
                    提案标题
                    <input
                      value={title}
                      onChange={(e) => setTitle(e.target.value)}
                      maxLength={200}
                    />
                  </label>
                  <p>选中照片：{photo?.filename}</p>
                  <div className="fr-inline">
                    {(["add", "move", "remove"] as const).map((op) => (
                      <button
                        key={op}
                        disabled={!canWrite || !photo}
                        onClick={() =>
                          setOperations((prev) => [
                            ...prev,
                            {
                              op,
                              operation_id: crypto.randomUUID(),
                              photo_id: selected,
                              ...(op !== "remove"
                                ? {
                                    to_index:
                                      op === "add"
                                        ? state.snapshot.sequences[0].items
                                            .length
                                        : 0,
                                  }
                                : {}),
                            },
                          ])
                        }
                      >
                        {op === "add"
                          ? "追加到序列"
                          : op === "move"
                            ? "移到开头"
                            : "从序列移除"}
                      </button>
                    ))}
                  </div>
                  {operations.map((op) => (
                    <div className="fr-draft-op" key={op.operation_id}>
                      <span>
                        {op.op} ·{" "}
                        {
                          state.snapshot.photos.find(
                            (p) => p.id === op.photo_id,
                          )?.filename
                        }
                      </span>
                      <button
                        aria-label="移除草案操作"
                        onClick={() =>
                          setOperations((prev) =>
                            prev.filter(
                              (o) => o.operation_id !== op.operation_id,
                            ),
                          )
                        }
                      >
                        ×
                      </button>
                    </div>
                  ))}
                  <button
                    className="fr-primary"
                    disabled={!canWrite || !title.trim() || !operations.length}
                    onClick={() => void saveProposal(true)}
                  >
                    提交编辑提案
                  </button>
                  <button
                    disabled={!canWrite || !title.trim() || !operations.length}
                    onClick={() => void saveProposal(false)}
                  >
                    保存提案草案
                  </button>
                </section>
              )}
            </div>
          )}
          {tab === "access" && (
            <AccessPanel
              state={state}
              session={session}
              refresh={refresh}
              setError={setError}
            />
          )}
          {tab === "history" && (
            <HistoryPanel id={id} session={session} revision={state.revision} />
          )}
        </aside>
      </main>
    </>
  );
}
