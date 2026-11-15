const API_BASE =
  process.env.NEXT_PUBLIC_API_URL ?? `http://${window.location.hostname}:8000`;

export const WS_URL =
  process.env.NEXT_PUBLIC_WS_URL ?? `ws://${window.location.hostname}:8000/ws`;

async function post(path: string, body?: ArrayBuffer) {
  const res = await fetch(`${API_BASE}${path}`, { method: "POST", body });
  if (!res.ok) throw new Error(`${res.status}: ${await res.text()}`);
}

export const api = {
  uploadBot: (name: string, buf: ArrayBuffer) =>
    post(`/api/bots?name=${encodeURIComponent(name)}`, buf),
  spawnPreset: (kind: "hunter" | "sniper") => post(`/api/presets/${kind}`),
  reset: () => post("/api/reset"),
};
