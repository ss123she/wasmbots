use serde::Serialize;
use wasmtime::{Engine, Linker};

use crate::{
    geom,
    sandbox::{BotInput, BotRuntime, BotState, FUEL_PER_TICK, create_engine, create_linker},
};

pub const ARENA_SIZE: f32 = 1000.0;

// Internal.

pub struct BotEntity {
    pub id: usize,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub hp: f32,
    pub cooldown: f32,
    pub kills: u32,
    pub runtime: BotRuntime,
    pub last_scans: Vec<(f32, f32)>,
}

pub struct Bullet {
    pub owner_id: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub ttl: u32,
}

pub struct Orb {
    pub x: f32,
    pub y: f32,
    pub kind: &'static str,
}

pub struct World {
    engine: Engine,
    linker: Linker<BotState>,
    bots: Vec<BotEntity>,
    bullets: Vec<Bullet>,
    orbs: Vec<Orb>,
    tick: u64,
    next_id: usize,
}

// What sends to server.

#[derive(Serialize)]
pub struct BotView {
    pub id: usize,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub hp: f32,
    pub kills: u32,
    pub cpu_pct: f32,
    pub overheated: bool,
    pub scans: Vec<(f32, f32)>,
}

#[derive(Serialize)]
pub struct BulletView {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct OrbView {
    pub x: f32,
    pub y: f32,
    pub kind: &'static str,
}

#[derive(Serialize)]
pub struct ArenaSnapshot {
    pub tick: u64,
    pub bots: Vec<BotView>,
    pub bullets: Vec<BulletView>,
    pub orbs: Vec<OrbView>,
    pub events: Vec<String>,
}

impl World {
    pub fn new() -> anyhow::Result<Self> {
        let engine = create_engine()?;
        let linker = create_linker(&engine)?;
        Ok(Self {
            engine,
            linker,
            bots: Vec::new(),
            bullets: Vec::new(),
            orbs: Vec::new(),
            tick: 0,
            next_id: 1,
        })
    }

    pub fn spawn_bot(&mut self, name: &str, wasm_bytes: &[u8]) -> anyhow::Result<()> {
        let runtime = BotRuntime::from_bytes(&self.engine, &self.linker, wasm_bytes)?;
        let id = self.next_id;
        self.next_id += 1;

        let colors = [
            "#38bdf8", "#a855f7", "#f97316", "#ec4899", "#22d3ee", "#eab308",
        ];
        let color = colors[id % colors.len()].to_string();

        self.bots.push(BotEntity {
            id,
            name: name.to_string(),
            color,
            x: rand::random_range(100.0..900.0),
            y: rand::random_range(100.0..900.0),
            angle: rand::random_range(-180.0..180.0),
            hp: 100.0,
            cooldown: 0.0,
            kills: 0,
            runtime,
            last_scans: Vec::new(),
        });
        Ok(())
    }

    pub fn step(&mut self) -> ArenaSnapshot {
        self.tick += 1;
        let mut events = Vec::new();

        if self.tick % 120 == 0 && self.orbs.len() < 3 {
            self.orbs.push(Orb {
                x: rand::random_range(200.0..800.0),
                y: rand::random_range(200.0..800.0),
                kind: if rand::random_bool(0.5) {
                    "heal"
                } else {
                    "overclock"
                },
            });
        }

        let positions: Vec<(usize, f32, f32)> =
            self.bots.iter().map(|b| (b.id, b.x, b.y)).collect();

        for bot in &mut self.bots {
            bot.cooldown = (bot.cooldown - 1.0).max(0.0);

            let enemies: Vec<(f32, f32)> = positions
                .iter()
                .filter(|(id, _, _)| *id != bot.id)
                .map(|(_, x, y)| (*x, *y))
                .collect();

            let nearest_orb_angle = self
                .orbs
                .iter()
                .min_by(|a, b| {
                    geom::dist(bot.x, bot.y, a.x, a.y)
                        .total_cmp(&geom::dist(bot.x, bot.y, b.x, b.y))
                })
                .map(|o| {
                    geom::normalize_deg(geom::angle_to_deg(bot.x, bot.y, o.x, o.y) - bot.angle)
                })
                .unwrap_or(999.0);

            let out = bot.runtime.step(BotInput {
                hp: bot.hp,
                x: bot.x,
                y: bot.y,
                angle: bot.angle,
                cooldown: bot.cooldown,
                nearest_orb_angle,
                enemies,
            });

            bot.last_scans = out.scans;
            bot.angle = geom::normalize_deg(bot.angle + out.turn * 6.0);
            let rad = bot.angle.to_radians();
            bot.x = (bot.x + rad.cos() * out.speed * 5.0).clamp(20.0, ARENA_SIZE - 20.0);
            bot.y = (bot.y + rad.sin() * out.speed * 5.0).clamp(20.0, ARENA_SIZE - 20.0);

            if out.fired {
                bot.cooldown = 15.0;
                self.bullets.push(Bullet {
                    owner_id: bot.id,
                    x: bot.x + rad.cos() * 22.0,
                    y: bot.y + rad.sin() * 22.0,
                    vx: rad.cos() * 14.0,
                    vy: rad.sin() * 14.0,
                    ttl: 70,
                });
            }
        }

        let mut bullets_alive = Vec::with_capacity(self.bullets.len());
        for mut b in self.bullets.drain(..) {
            b.x += b.vx;
            b.y += b.vy;
            if b.ttl == 0 || b.x < 0.0 || b.x > ARENA_SIZE || b.y < 0.0 || b.y > ARENA_SIZE {
                continue;
            }
            b.ttl -= 1;

            let mut hit_owner = None;
            for bot in &mut self.bots {
                if bot.id != b.owner_id && geom::dist(b.x, b.y, bot.x, bot.y) < 20.0 {
                    bot.hp -= 20.0;
                    hit_owner = Some((b.owner_id, bot.name.clone(), bot.hp <= 0.0));

                    break;
                }
            }

            if let Some((killer_id, victim_name, died)) = hit_owner {
                if died {
                    if let Some(killer) = self.bots.iter_mut().find(|k| k.id == killer_id) {
                        killer.kills += 1;
                        events.push(format!("{} destroyed {}", killer.name, victim_name));
                    }
                }
            } else {
                bullets_alive.push(b);
            }
        }

        self.bullets = bullets_alive;

        self.orbs.retain(|orb| {
            for bot in &mut self.bots {
                if geom::dist(bot.x, bot.y, orb.x, orb.y) < 24.0 {
                    if orb.kind == "heal" {
                        bot.hp = (bot.hp + 35.0).min(100.0);
                    }
                    events.push(format!("{} picked {}", bot.name, orb.kind));
                    return false;
                }
            }
            true
        });

        // Killed bots. Respawned in random place.
        for bot in &mut self.bots {
            if bot.hp <= 0.0 {
                bot.hp = 100.0;
                bot.x = rand::random_range(100.0..900.0);
                bot.y = rand::random_range(100.0..900.0);
            }
        }

        ArenaSnapshot {
            tick: self.tick,
            bots: self
                .bots
                .iter()
                .map(|b| BotView {
                    id: b.id,
                    name: b.name.clone(),
                    color: b.color.clone(),
                    x: b.x,
                    y: b.y,
                    angle: b.angle,
                    hp: b.hp,
                    kills: b.kills,
                    cpu_pct: (b.runtime.last_fuel_used as f32 / FUEL_PER_TICK as f32) * 100.0,
                    overheated: b.runtime.overheated_ticks > 0,
                    scans: b.last_scans.clone(),
                })
                .collect(),
            bullets: self
                .bullets
                .iter()
                .map(|b| BulletView { x: b.x, y: b.y })
                .collect(),
            orbs: self
                .orbs
                .iter()
                .map(|o| OrbView {
                    x: o.x,
                    y: o.y,
                    kind: o.kind,
                })
                .collect(),
            events,
        }
    }
}
