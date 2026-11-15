"use client";
import { useCallback, useEffect, useState } from "react";
import { getWsUrl } from "./api";
import { ArenaSnapshot } from "./types";

export function useArena() {
  const [snapshot, setSnapshot] = useState<ArenaSnapshot | null>(null);
  const [events, setEvents] = useState<string[]>([]);
  const [wsStatus, setWsStatus] = useState("connecting...");
  const [err, setErr] = useState("");

  useEffect(() => {
    let disposed = false;
    let ws: WebSocket | null = null;
    let timer: ReturnType<typeof setTimeout>;

    const connect = () => {
      if (disposed) return;
      setWsStatus("connecting...");
      const sock = new WebSocket(getWsUrl());
      ws = sock;

      sock.onopen = () => {
        if (disposed) {
          sock.close();
          return;
        }
        setWsStatus("connected");
      };
      sock.onerror = () => !disposed && setWsStatus("error");
      sock.onclose = () => {
        if (disposed) return;
        setWsStatus("closed");
        timer = setTimeout(connect, 1500);
      };
      sock.onmessage = (ev) => {
        if (disposed) return;
        const data: ArenaSnapshot = JSON.parse(ev.data);
        setSnapshot(data);
        if (data.events?.length) {
          setEvents((prev) => [...data.events, ...prev].slice(0, 12));
        }
      };
    };

    connect();

    return () => {
      disposed = true;
      clearTimeout(timer);
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    };
  }, []);

  const run = useCallback(async (action: () => Promise<void>) => {
    try {
      setErr("");
      await action();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const clearEvents = useCallback(() => setEvents([]), []);

  return { snapshot, events, wsStatus, err, run, clearEvents };
}
