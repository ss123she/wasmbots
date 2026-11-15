export type { ArenaSnapshot } from "../bindings/ArenaSnapshot";
export type { BotView } from "../bindings/BotView";
export type { BulletView } from "../bindings/BulletView";
export type { ModuleView } from "../bindings/ModuleView";
export type { OrbView } from "../bindings/OrbView";

export const ModuleKind = {
  Core: 0,
  Armor: 1,
  Thruster: 2,
  Cannon: 3,
  Radar: 4,
  Reactor: 5,
} as const;

export type ModuleKind = (typeof ModuleKind)[keyof typeof ModuleKind];
