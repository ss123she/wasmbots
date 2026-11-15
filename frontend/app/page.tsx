"use client";
import { useEffect, useState } from "react";
import ArenaScene from "./scene";
import { ArenaSnapshot } from "./types";

function getApiBase() {
  return (
    process.env.NEXT_PUBLIC_API_URL ?? `http://${window.location.hostname}:8000`
  );
}

export default function ArenaPage() {
  const [snapshot, setSnapshot] = useState<ArenaSnapshot | null>(null);
  const [followId, setFollowId] = useState<number | null>(null);
  const [name, setName] = useState("MyBot");
  const [events, setEvents] = useState<string[]>([]);
  const [wsStatus, setWsStatus] = useState("connecting...");
  const [err, setErr] = useState("");

  useEffect(() => {
    const wsUrl =
      process.env.NEXT_PUBLIC_WS_URL ??
      `ws://${window.location.hostname}:8000/ws`;

    const ws = new WebSocket(wsUrl);
    ws.onopen = () => setWsStatus("connected");
    ws.onerror = () => setWsStatus("error");
    ws.onclose = () => setWsStatus("closed");

    ws.onmessage = (ev) => {
      const data: ArenaSnapshot = JSON.parse(ev.data);
      setSnapshot(data);
      if (data.events?.length) {
        setEvents((prev) => [...data.events, ...prev].slice(0, 12));
      }
    };

    return () => ws.close();
  }, []);

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      setErr("");
      const buf = await file.arrayBuffer();
      const res = await fetch(
        `${getApiBase()}/api/bots?name=${encodeURIComponent(name)}`,
        { method: "POST", body: buf },
      );
      if (!res.ok) setErr(`${res.status}: ${await res.text()}`);
    } catch (e) {
      setErr(String(e));
    } finally {
      e.target.value = "";
    }
  }

  async function spawnPreset(kind: "hunter" | "sniper") {
    try {
      setErr("");
      const res = await fetch(`${getApiBase()}/api/presets/${kind}`, {
        method: "POST",
      });
      if (!res.ok) setErr(`${res.status}: ${await res.text()}`);
    } catch (e) {
      setErr(String(e));
    }
  }

  async function resetArena() {
    try {
      setErr("");
      const res = await fetch(`${getApiBase()}/api/reset`, { method: "POST" });
      if (!res.ok) setErr(`${res.status}: ${await res.text()}`);
      setEvents([]);
      setFollowId(null);
    } catch (e) {
      setErr(String(e));
    }
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100 p-6 flex flex-col lg:flex-row gap-6">
      {/* 3D Area */}
      <div className="flex-1 h-162.5 lg:h-[calc(100vh-3rem)]">
        <ArenaScene snapshot={snapshot} followBotId={followId} />
      </div>

      {/* Sidebar */}
      <aside className="w-full lg:w-96 shrink-0 flex flex-col gap-4 font-mono">
        <div className="flex items-center justify-between">
          <h1 className="text-2x1 font-bold tracking-tight">WASM ARENA</h1>
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

        {/* Load .wasm */}
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
              onClick={() => spawnPreset("hunter")}
              className="flex-1 bg-sky-600 hover:bg-sky-500 px-3 py-1.5 rounded text-sm font-semibold cursor-pointer"
            >
              + Hunter
            </button>
            <button
              onClick={() => spawnPreset("sniper")}
              className="flex-1 bg-purple-600 hover:bg-purple-500 px-3 py-1.5 rounded text-sm font-semibold cursor-pointer"
            >
              + Sniper
            </button>
            <button
              onClick={resetArena}
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
