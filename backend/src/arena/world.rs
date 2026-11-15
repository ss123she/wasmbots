use wasmtime::{Engine, Linker};

use crate::{
    arena::{
        geom,
        modules::{CELL_SIZE, Module, ModuleKind, compute_mobility, count_alive, min_cannon_cd},
        snapshot::{ArenaSnapshot, BotView, BulletView, OrbView},
    },
    wasm::{
        host::create_linker,
        sandbox::{BASE_FUEL_PER_TICK, BotInput, BotRuntime, BotState, create_engine},
    },
};

pub const ARENA_SIZE: f32 = 1000.0;

fn random_spawn() -> (f32, f32) {
    (
        rand::random_range(100.0..900.0),
        rand::random_range(100.0..900.0),
    )
}

pub struct BotEntity {
    pub id: usize,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub kills: u32,
    pub overclock_ticks: u32,
    pub modules: Vec<Module>,
    pub runtime: BotRuntime,
    pub last_scans: Vec<(f32, f32)>,
}

impl BotEntity {
    fn core_hp(&self) -> f32 {
        self.modules.first().map(|m| m.hp).unwrap_or(0.0)
    }

    fn to_view(&self) -> BotView {
        BotView {
            id: self.id,
            name: self.name.clone(),
            color: self.color.clone(),
            x: self.x,
            y: self.y,
            angle: self.angle,
            hp: self.core_hp().max(0.0),
            kills: self.kills,
            cpu_pct: (self.runtime.last_fuel_used as f32 / self.runtime.last_fuel_budget as f32)
                * 100.0,
            overheated: self.runtime.overheated_ticks > 0,
            scans: self.last_scans.clone(),
            modules: self.modules.iter().map(Module::to_view).collect(),
        }
    }
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
        let modules = runtime.blueprint.clone();
        let id = self.next_id;
        self.next_id += 1;

        let colors = [
            "#38bdf8", "#a855f7", "#f97316", "#ec4899", "#22d3ee", "#eab308",
        ];
        let (x, y) = random_spawn();

        self.bots.push(BotEntity {
            id,
            name: name.to_string(),
            color: colors[id % colors.len()].to_string(),
            x,
            y,
            angle: rand::random_range(-180.0..180.0),
            kills: 0,
            overclock_ticks: 0,
            modules,
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

        self.step_bots();
        self.step_bullets(&mut events);
        self.step_orbs(&mut events);

        for bot in &mut self.bots {
            if bot.core_hp() <= 0.0 {
                bot.modules.iter_mut().for_each(Module::reset);
                bot.overclock_ticks = 0;
                (bot.x, bot.y) = random_spawn();
            }
        }

        ArenaSnapshot {
            tick: self.tick,
            bots: self.bots.iter().map(BotEntity::to_view).collect(),
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

    fn step_bots(&mut self) {
        let positions: Vec<(usize, f32, f32)> =
            self.bots.iter().map(|b| (b.id, b.x, b.y)).collect();

        for bot in &mut self.bots {
            bot.overclock_ticks = bot.overclock_ticks.saturating_sub(1);
            let reactors = count_alive(&bot.modules, ModuleKind::Reactor) as u64;
            let cd_step =
                1.0 + (reactors as f32 * 0.15) + if bot.overclock_ticks > 0 { 0.8 } else { 0.0 };

            for m in &mut bot.modules {
                m.cooldown = (m.cooldown - cd_step).max(0.0);
            }

            let enemies = positions
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

            let fuel_budget = BASE_FUEL_PER_TICK
                + reactors * 3_000
                + if bot.overclock_ticks > 0 { 10_000 } else { 0 };

            let out = bot.runtime.step(
                BotInput {
                    hp: bot.core_hp(),
                    x: bot.x,
                    y: bot.y,
                    angle: bot.angle,
                    cooldown: min_cannon_cd(&bot.modules),
                    nearest_orb_angle,
                    enemies,
                    modules: bot.modules.clone(),
                },
                fuel_budget,
            );

            let (speed_mul, turn_mul) = compute_mobility(&bot.modules);
            bot.last_scans = out.scans;
            bot.angle = geom::normalize_deg(bot.angle + out.turn * 5.0 * turn_mul);
            let (s, c) = bot.angle.to_radians().sin_cos();
            bot.x = (bot.x + c * out.speed * 4.5 * speed_mul).clamp(30.0, ARENA_SIZE - 30.0);
            bot.y = (bot.y + s * out.speed * 4.5 * speed_mul).clamp(30.0, ARENA_SIZE - 30.0);

            for idx in out.fired_modules {
                let (bx, by, angle) = (bot.x, bot.y, bot.angle);
                if let Some(m) = bot.modules.get_mut(idx).filter(|m| m.is_ready_cannon()) {
                    m.cooldown = 15.0;
                    let (mx, my) = m.world_pos(bx, by, angle);
                    let (fs, fc) = (angle + m.dir).to_radians().sin_cos();
                    self.bullets.push(Bullet {
                        owner_id: bot.id,
                        x: mx + fc * 12.0,
                        y: my + fs * 12.0,
                        vx: fc * 14.0,
                        vy: fs * 14.0,
                        ttl: 70,
                    });
                }
            }
        }
    }

    fn step_bullets(&mut self, events: &mut Vec<String>) {
        let mut alive = Vec::with_capacity(self.bullets.len());
        for mut b in self.bullets.drain(..) {
            b.x += b.vx;
            b.y += b.vy;
            if b.ttl == 0 || b.x < 0.0 || b.x > ARENA_SIZE || b.y < 0.0 || b.y > ARENA_SIZE {
                continue;
            }
            b.ttl -= 1;

            let mut hit = None;
            for bot in &mut self.bots {
                if bot.id == b.owner_id || geom::dist(b.x, b.y, bot.x, bot.y) > 40.0 {
                    continue;
                }
                let (lx, ly) = geom::world_to_local(bot.x, bot.y, bot.angle, b.x, b.y);
                let hit_mod = bot
                    .modules
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.is_alive())
                    .filter_map(|(idx, m)| {
                        let d =
                            geom::dist(lx, ly, m.gx as f32 * CELL_SIZE, m.gy as f32 * CELL_SIZE);
                        (d <= CELL_SIZE * 0.68).then_some((idx, d))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1));

                if let Some((idx, _)) = hit_mod {
                    bot.modules[idx].hp -= 20.0;
                    let core_dead = bot.core_hp() <= 0.0;
                    let broken = !bot.modules[idx].is_alive() && !core_dead;
                    hit = Some((
                        b.owner_id,
                        bot.name.clone(),
                        core_dead,
                        broken,
                        bot.modules[idx].kind,
                    ));
                    break;
                }
            }

            if let Some((killer_id, victim, core_dead, broken, kind)) = hit {
                if core_dead {
                    if let Some(k) = self.bots.iter_mut().find(|k| k.id == killer_id) {
                        k.kills += 1;
                        events.push(format!("{} destroyed {}", k.name, victim));
                    }
                } else if broken {
                    events.push(format!("{victim} lost {kind:?}"));
                }
            } else {
                alive.push(b);
            }
        }
        self.bullets = alive;
    }

    fn step_orbs(&mut self, events: &mut Vec<String>) {
        self.orbs.retain(|orb| {
            for bot in &mut self.bots {
                if geom::dist(bot.x, bot.y, orb.x, orb.y) < 28.0 {
                    if orb.kind == "heal" {
                        for m in &mut bot.modules {
                            m.hp = (m.hp + m.max_hp * 0.45).min(m.max_hp);
                        }
                    } else {
                        bot.overclock_ticks = 180;
                    }
                    events.push(format!("{} picked {}", bot.name, orb.kind));
                    return false;
                }
            }
            true
        });
    }
}
