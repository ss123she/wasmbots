"use client";
import { useEffect, useRef, useState } from "react";

interface BotView {
  id: number;
  name: string;
  color: string;
  x: number;
  y: number;
  angle: number;
  hp: number;
  kills: number;
  cpu_pct: number;
  overheated: boolean;
  scans: [number, number][];
}

interface BulletView {
  x: number;
  y: number;
}

interface OrbView {
  x: number;
  y: number;
  kind: string;
}

interface ArenaSnapshot {
  tick: number;
  bots: BotView[];
  bullets: BulletView[];
  orbs: OrbView[];
  events: string[];
}

export default function ArenaPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [name, setName] = useState("Bot");
  const [stats, setStats] = useState<BotView[]>([]);
  const [events, setEvents] = useState<string[]>([]);

  useEffect(() => {
    const host = window.location.hostname;
    const ws = new WebSocket(`ws://${host}:8000/ws`);

    ws.onmessage = (ev) => {
      const data: ArenaSnapshot = JSON.parse(ev.data);
      setStats(data.bots);
      if (data.events?.length) {
        setEvents((prev) => [...data.events, ...prev].slice(0, 10));
      }

      const canvas = canvasRef.current;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx) return;

      ctx.fillStyle = "#09090b";
      ctx.fillRect(0, 0, 1000, 1000);

      for (const orb of data.orbs) {
        ctx.fillStyle = orb.kind === "overclock" ? "#facc15" : "#4ade80";
        ctx.beginPath();
        ctx.arc(orb.x, orb.y, 10, 0, Math.PI * 2);
        ctx.fill();
      }

      ctx.fillStyle = "#f43f5e";
      for (const b of data.bullets) {
        ctx.beginPath();
        ctx.arc(b.x, b.y, 4, 0, Math.PI * 2);
        ctx.fill();
      }

      for (const bot of data.bots) {
        const rad = (bot.angle * Math.PI) / 180;

        for (const [offset, fov] of bot.scans) {
          const start = ((bot.angle + offset - fov / 2) * Math.PI) / 180;
          const end = ((bot.angle + offset + fov / 2) * Math.PI) / 180;

          ctx.fillStyle = "rgba(56, 189, 248, 0.12)";
          ctx.beginPath();
          ctx.moveTo(bot.x, bot.y);
          ctx.arc(bot.x, bot.y, 350, start, end);
          ctx.closePath();
          ctx.fill();
        }

        ctx.save();
        ctx.translate(bot.x, bot.y);
        ctx.rotate(rad);
        ctx.fillStyle = bot.overheated ? "#ef4444" : bot.color;
        ctx.beginPath();
        ctx.moveTo(18, 0);
        ctx.lineTo(-12, -12);
        ctx.lineTo(-8, 0);
        ctx.lineTo(-12, 12);
        ctx.closePath();
        ctx.fill();
        ctx.restore();

        ctx.fillStyle = "#e4e4e7";
        ctx.font = "12px monospace";
        ctx.fillText(
          `${bot.name} | HP:${Math.round(bot.hp)} | CPU:${Math.round(bot.cpu_pct)}%`,
          bot.x - 40,
          bot.y - 24,
        );
      }
    };

    return () => ws.close();
  }, []);

  async function handleUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const apiBase = `http://${window.location.hostname}:8000`;
    const file = e.target.files?.[0];
    if (!file) return;
    const buf = await file.arrayBuffer();
    await fetch(`${apiBase}/api/bots?name=${encodeURIComponent(name)}`, {
      method: "POST",
      body: buf,
    });
  }

  async function spawnPreset(kind: "hunter" | "sniper") {
    await fetch(`http://${window.location.hostname}:8000/api/presets/${kind}`, {
      method: "POST",
    });
  }

  async function resetArena() {
    await fetch(`http://${window.location.hostname}:8000/api/reset`, {
      method: "POST",
    });
  }

  return (
    <main className="min-h-screen big-zinc-950 text-zinc-100 p-6 flex gap-6">
      <canvas
        ref={canvasRef}
        width={1000}
        height={1000}
        className="w-175 border border-zin-800 rounded-lg bg-black"
      />
      <div className="flex-1 flex flex-col gap-4">
        <h1 className="text-2x1 font-bold font-mono">WASM ARENA</h1>
        <div className="flex gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="bg-zinc-900 border border-zinc-700 px-3 py-1 rounder"
            placeholder="Bot name"
          />
          <input type="file" accept=".wasm" onChange={handleUpload} />
          <div className="flex gap-2">
            <button
              onClick={() => spawnPreset("hunter")}
              className="bg-sky-600 px-3 py-1 rounded"
            >
              + Hunter
            </button>
            <button
              onClick={() => spawnPreset("sniper")}
              className="bg-purple-600 px-3 py-1 rounded"
            >
              + Sniper
            </button>
            <button
              onClick={resetArena}
              className="bg-red-700 px-3 py-1 rounded"
            >
              Reset
            </button>
          </div>
        </div>
      </div>
    </main>
  );
}
