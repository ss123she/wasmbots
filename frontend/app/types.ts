export const ModuleKind = {
  Core: 0,
  Armor: 1,
  Thruster: 2,
  Cannon: 3,
  Radar: 4,
  Reactor: 5,
} as const;

export type ModuleKind = (typeof ModuleKind)[keyof typeof ModuleKind];

export interface ModuleView {
  gx: number;
  gy: number;
  kind: ModuleKind;
  dir: number;
  hp_pct: number;
}

export interface BotView {
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
  modules: ModuleView[];
}

export interface BulletView {
  x: number;
  y: number;
}

export type OrbKind = "heal" | "overclock";

export interface OrbView {
  x: number;
  y: number;
  kind: OrbKind;
}

export interface ArenaSnapshot {
  ticks: number;
  bots: BotView[];
  bullets: BulletView[];
  orbs: OrbView[];
  events: string[];
}
