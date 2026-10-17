import { useEffect, useRef, useState } from "react";
import * as Y from "yjs";
import { CloudError, request, type Session } from "../shared-api";
export function useRealtime(
  id: string,
  session: Session,
  refresh: () => Promise<void>,
  revoke: () => void,
) {
  const [online, setOnline] = useState(false),
    [people, setPeople] = useState<
      { id: string; name: string; photo_id: string | null }[]
    >([]),
    [note, setNote] = useState("");
  const socket = useRef<WebSocket | null>(null),
    document = useRef<Y.Doc | null>(null);
  useEffect(() => {
    let disposed = false,
      retry: ReturnType<typeof setTimeout> | undefined;
    const doc = new Y.Doc();
    document.current = doc;
    const connect = async () => {
      try {
        const ticket = await request<{ token: string }>(
          `/projects/${id}/ws-ticket`,
          session,
          "POST",
        );
        if (disposed) return;
        const ws = new WebSocket(
          `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/api/v2/realtime`,
        );
        socket.current = ws;
        ws.onopen = () =>
          ws.send(
            JSON.stringify({ type: "authenticate", ticket: ticket.token }),
          );
        ws.onmessage = (event) => {
          try {
            const value = JSON.parse(event.data);
            if (value.type === "sync") {
              setOnline(true);
              if (value.draft)
                Y.applyUpdate(
                  doc,
                  Uint8Array.from(atob(value.draft), (c) => c.charCodeAt(0)),
                );
              setNote(doc.getText("note").toString());
              void refresh().catch(() => {});
            }
            if (
              value.type === "events" ||
              value.type === "resync" ||
              value.type === "access.changed"
            )
              void refresh().catch(() => {});
            if (value.type === "presence") setPeople(value.users);
            if (value.type === "draft") {
              Y.applyUpdate(
                doc,
                Uint8Array.from(atob(value.update), (c) => c.charCodeAt(0)),
              );
              setNote(doc.getText("note").toString());
            }
          } catch {
            ws.close();
          }
        };
        ws.onclose = (event) => {
          setOnline(false);
          setPeople([]);
          if (disposed) return;
          if (event.code === 4403) {
            revoke();
            return;
          }
          retry = setTimeout(() => void connect(), 1500);
        };
        ws.onerror = () => ws.close();
      } catch (error) {
        if (!disposed) {
          setOnline(false);
          if (
            error instanceof CloudError &&
            ["AUTH_REVOKED", "PERMISSION_DENIED"].includes(error.code)
          )
            revoke();
          else retry = setTimeout(() => void connect(), 2000);
        }
      }
    };
    void connect();
    const heartbeat = setInterval(() => {
      if (socket.current?.readyState === WebSocket.OPEN)
        socket.current.send(JSON.stringify({ type: "heartbeat" }));
    }, 10000);
    return () => {
      disposed = true;
      clearInterval(heartbeat);
      clearTimeout(retry);
      socket.current?.close();
      doc.destroy();
      document.current = null;
    };
  }, [id, session, refresh, revoke]);
  const updateNote = (value: string) => {
    const doc = document.current,
      ws = socket.current;
    if (!doc || !online || ws?.readyState !== WebSocket.OPEN) return;
    const before = Y.encodeStateVector(doc);
    doc.transact(() => {
      const text = doc.getText("note");
      text.delete(0, text.length);
      text.insert(0, value);
    });
    setNote(value);
    ws.send(
      JSON.stringify({
        type: "draft",
        update: btoa(
          String.fromCharCode(...Y.encodeStateAsUpdate(doc, before)),
        ),
      }),
    );
  };
  return {
    online,
    people,
    note,
    updateNote,
    viewPhoto: (photoId: string) => {
      if (online && socket.current?.readyState === WebSocket.OPEN)
        socket.current.send(
          JSON.stringify({ type: "presence", photo_id: photoId }),
        );
    },
  };
}
