"use client";
import { useState } from "react";
import { api } from "./lib/api";
import { useArena } from "./lib/use-arena";
import ArenaScene from "./scene/ArenaScene";

export default function ArenaPage() {
  const { snapshot, wsStatus, err, run, clearEvents } = useArena();
  const [followId, setFollowId] = useState<number | null>(null);
  const [name, setName] = useState("MyBot");

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file) {
      await run(async () => api.uploadBot(name, await file.arrayBuffer()));
    }
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
                  clearEvents();
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
