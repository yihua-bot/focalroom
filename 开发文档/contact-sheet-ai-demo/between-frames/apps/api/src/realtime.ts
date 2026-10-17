import { WebSocketServer, WebSocket } from "ws";
import type { Server } from "node:http";
import * as Y from "yjs";
import type { CloudService } from "./service.ts";
import { assert } from "./errors.ts";
import { transaction } from "./database.ts";
import type { Identity } from "./auth.ts";
import type { Project } from "./authority.ts";
type Peer = {
  ws: WebSocket;
  user: Identity;
  project: Project;
  cursor: string;
  lastSeen: number;
  viewing: string | null;
  doc: Y.Doc;
  count: number;
  window: number;
  access: string;
};
export function attachRealtime(server: Server, service: CloudService) {
  const wss = new WebSocketServer({
    noServer: true,
    maxPayload: 65536,
    perMessageDeflate: false,
  });
  const peers = new Set<Peer>();
  server.on("upgrade", (request, socket, head) => {
    try {
      service.auth.checkOrigin(request.headers);
      assert(
        request.url === "/api/v2/realtime",
        404,
        "NOT_FOUND",
        "通道不存在",
      );
      wss.handleUpgrade(request, socket, head, (ws) =>
        wss.emit("connection", ws),
      );
    } catch {
      socket.end("HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n");
    }
  });
  const send = (ws: WebSocket, data: unknown) => {
    if (ws.readyState === WebSocket.OPEN && ws.bufferedAmount < 262144)
      ws.send(JSON.stringify(data));
    else ws.close(1013, "slow consumer");
  };
  const live = async (peer: Peer) => {
    peer.user = await service.identityForSession(peer.user.session);
    const project = (
      await service.pool.query(
        "SELECT * FROM fr_projects WHERE id=$1 AND state='SHARED'",
        [peer.project.id],
      )
    ).rows[0];
    assert(project, 403, "AUTH_REVOKED", "项目不可访问");
    const principal = await service.authority.principal(
      service.pool,
      peer.user,
      project,
    );
    peer.project = project;
    const access = JSON.stringify(principal);
    if (peer.access !== access) {
      peer.access = access;
      send(peer.ws, { type: "access.changed" });
    }
    return principal;
  };
  wss.on("connection", (ws) => {
    let peer: Peer | undefined;
    let busy = false;
    const timeout = setTimeout(
      () => ws.close(4401, "authentication timeout"),
      5000,
    );
    ws.on("message", async (raw, binary) => {
      if (busy) {
        ws.close(4408, "overlapping messages");
        return;
      }
      busy = true;
      try {
        assert(!binary, 400, "VALIDATION_ERROR", "仅支持 JSON 消息");
        const message = JSON.parse(raw.toString()) as Record<string, unknown>;
        if (!peer) {
          assert(
            message.type === "authenticate" &&
              typeof message.ticket === "string",
            401,
            "UNAUTHORIZED",
            "先提交一次性连接票据",
          );
          const result = await service.useTicket(message.ticket, "WS");
          const doc = new Y.Doc();
          const saved = (
            await service.pool.query(
              "SELECT document FROM fr_drafts WHERE project_id=$1 AND user_id=$2",
              [result.project.id, result.user.id],
            )
          ).rows[0];
          if (saved) Y.applyUpdate(doc, saved.document);
          const cursor = (
            await service.pool.query(
              "SELECT coalesce(max(id),0)::text AS cursor FROM fr_events WHERE project_id=$1",
              [result.project.id],
            )
          ).rows[0].cursor;
          peer = {
            ws,
            user: result.user,
            project: result.project,
            cursor,
            lastSeen: Date.now(),
            viewing: null,
            doc,
            count: 0,
            window: Date.now(),
            access: "",
          };
          peers.add(peer);
          clearTimeout(timeout);
          send(ws, {
            type: "sync",
            revision: peer.project.revision,
            cursor,
            draft: Buffer.from(Y.encodeStateAsUpdate(doc)).toString("base64"),
          });
          return;
        }
        const role = await live(peer);
        if (Date.now() - peer.window > 10000) {
          peer.window = Date.now();
          peer.count = 0;
        }
        assert(++peer.count <= 60, 429, "RATE_LIMITED", "消息过于频繁");
        peer.lastSeen = Date.now();
        if (message.type === "presence") {
          assert(
            message.photo_id === null ||
              (typeof message.photo_id === "string" &&
                peer.project.snapshot.photos.some(
                  (p) => p.id === message.photo_id,
                )),
            400,
            "VALIDATION_ERROR",
            "照片不属于此项目",
          );
          peer.viewing = message.photo_id as string | null;
        } else if (message.type === "draft") {
          assert(
            role.role !== "VIEWER" &&
              typeof message.update === "string" &&
              message.update.length <= 87384,
            403,
            "PERMISSION_DENIED",
            "无权更新私人草稿",
          );
          const current = peer;
          const bytes = await transaction(service.pool, async (tx) => {
            const user = await service.identityForSession(
              current.user.session,
              tx,
            );
            await tx.query(
              "SELECT id FROM fr_workspaces WHERE id=$1 FOR SHARE",
              [current.project.workspace_id],
            );
            const project = (
              await tx.query(
                "SELECT * FROM fr_projects WHERE id=$1 FOR UPDATE",
                [current.project.id],
              )
            ).rows[0];
            const access = await service.authority.principal(tx, user, project);
            assert(
              access.role !== "VIEWER",
              403,
              "PERMISSION_DENIED",
              "只读角色不能写草稿",
            );
            const saved = (
              await tx.query(
                "SELECT document FROM fr_drafts WHERE project_id=$1 AND user_id=$2",
                [project.id, user.id],
              )
            ).rows[0];
            const candidate = new Y.Doc();
            try {
              if (saved) Y.applyUpdate(candidate, saved.document);
              Y.applyUpdate(
                candidate,
                Buffer.from(message.update as string, "base64"),
              );
              assert(
                [...candidate.share.keys()].every((key) => key === "note") &&
                  candidate.getText("note").length <= 4000,
                400,
                "DRAFT_SCOPE",
                "CRDT 仅允许私人笔记",
              );
              const encoded = Buffer.from(Y.encodeStateAsUpdate(candidate));
              assert(
                encoded.length <= 65536,
                400,
                "DRAFT_LIMIT",
                "草稿大小超限",
              );
              await tx.query(
                "INSERT INTO fr_drafts(project_id,user_id,document) VALUES($1,$2,$3) ON CONFLICT(project_id,user_id) DO UPDATE SET document=excluded.document,updated_at=now()",
                [project.id, user.id, encoded],
              );
              return encoded;
            } finally {
              candidate.destroy();
            }
          });
          for (const other of peers)
            if (
              other.project.id === current.project.id &&
              other.user.id === current.user.id
            ) {
              await live(other);
              Y.applyUpdate(other.doc, bytes);
              send(other.ws, {
                type: "draft",
                update: bytes.toString("base64"),
              });
            }
        } else if (message.type !== "heartbeat")
          throw new Error("unsupported message");
      } catch {
        ws.close(4403, "authorization or message rejected");
      } finally {
        busy = false;
      }
    });
    ws.on("close", () => {
      clearTimeout(timeout);
      if (peer) {
        peers.delete(peer);
        peer.doc.destroy();
      }
    });
    ws.on("error", () => ws.close());
  });
  let ticking = false;
  const timer = setInterval(async () => {
    if (ticking) return;
    ticking = true;
    try {
      for (const peer of peers) {
        try {
          assert(
            Date.now() - peer.lastSeen < 30000,
            401,
            "EXPIRED",
            "心跳已过期",
          );
          await live(peer);
          const events = (
            await service.pool.query(
              "SELECT id::text,kind,revision FROM fr_events WHERE project_id=$1 AND id>$2 ORDER BY id LIMIT 101",
              [peer.project.id, peer.cursor],
            )
          ).rows;
          if (events.length > 100)
            send(peer.ws, { type: "resync", revision: peer.project.revision });
          else if (events.length) send(peer.ws, { type: "events", events });
          if (events.length) peer.cursor = events[events.length - 1].id;
        } catch {
          peer.ws.close(4403, "access revoked");
          peers.delete(peer);
        }
      }
      for (const peer of peers)
        send(peer.ws, {
          type: "presence",
          users: [...peers]
            .filter((p) => p.project.id === peer.project.id)
            .map((p) => ({
              id: p.user.id,
              name: p.user.name,
              photo_id: p.viewing,
            })),
        });
    } finally {
      ticking = false;
    }
  }, 500);
  timer.unref();
  return async () => {
    clearInterval(timer);
    for (const peer of peers) peer.ws.terminate();
    await new Promise<void>((resolve) => wss.close(() => resolve()));
  };
}
