"use client";
import { useEffect, useState } from "react";
import ArenaScene from "./scene";
import { ArenaSnapshot } from "./types";
import { api, getWsUrl } from "./api";

export default function ArenaPage() {
  const [snapshot, setSnapshot] = useState<ArenaSnapshot | null>(null);
  const [followId, setFollowId] = useState<number | null>(null);
  const [name, setName] = useState("MyBot");
  const [, setEvents] = useState<string[]>([]);
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

  async function run(action: () => Promise<void>) {
    try {
      setErr("");
      await action();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file)
      await run(async () => api.uploadBot(name, await file.arrayBuffer()));
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100 p-6 flex flex-col lg:flex-row gap-6">
      <div className="flex-1 h-162.5 lg:h-[calc(100vh-3rem)]">
        <ArenaScene snapshot={snapshot} followBotId={followId} />
      </div>

      <aside className="w-full lg:w-96 shrink-0 flex flex-col gap-4 font-mono">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold tracking-tight">WASM ARENA</h1>
          <span
            className={`text-xs px-2 py-0.5 rounded ${
              wsStatus === "connected"
                ? "bg-emerald-950 text-emerald-400 border border-emerald-800"
                : "bg-red-950 text-red-400 border border-red-800"
            }`}
          >
            WS: {wsStatus}
          </span>
        </div>

        {err && (
          <div className="text-xs bg-red-950/70 border border-red-800 text-red-300 p-2 rounded">
            {err}
          </div>
        )}

        <div className="bg-zinc-900/70 border border-zinc-800 p-3 rounded-lg flex flex-col gap-2">
          <div className="flex gap-2">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="flex-1 bg-zinc-950 border border-zinc-700 px-3 py-1.5 rounded text-sm"
              placeholder="Bot name"
            />
            <label className="bg-zinc-800 hover:bg-zinc-700 cursor-pointer px-3 py-1.5 rounded text-sm flex items-center">
              Upload .wasm
              <input
                type="file"
                accept=".wasm"
                onChange={handleUpload}
                className="hidden"
              />
            </label>
          </div>

          <div className="flex gap-2">
            <button
              onClick={() => run(() => api.spawnPreset("hunter"))}
              className="flex-1 bg-sky-600 hover:bg-sky-500 px-3 py-1.5 rounded text-sm font-semibold cursor-pointer"
            >
              + Hunter
            </button>
            <button
              onClick={() => run(() => api.spawnPreset("sniper"))}
              className="flex-1 bg-purple-600 hover:bg-purple-500 px-3 py-1.5 rounded text-sm font-semibold cursor-pointer"
            >
              + Sniper
            </button>
            <button
              onClick={() =>
                run(async () => {
                  await api.reset();
                  setEvents([]);
                  setFollowId(null);
                })
              }
              className="bg-red-700 hover:bg-red-600 px-3 py-1.5 rounded text-sm font-semibold cursor-pointer"
            >
              Reset
            </button>
          </div>
        </div>
      </aside>
    </main>
  );
}
