use glam::Vec2;
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

fn random_pos(min: f32, max: f32) -> Vec2 {
    Vec2::new(rand::random_range(min..max), rand::random_range(min..max))
}

pub struct BotEntity {
    pub id: usize,
    pub name: String,
    pub color: String,
    pub pos: Vec2,
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

    fn total_hp(&self) -> f32 {
        self.modules.iter().map(|m| m.hp.max(0.0)).sum()
    }

    fn max_total_hp(&self) -> f32 {
        self.modules.iter().map(|m| m.max_hp).sum()
    }

    fn is_dead(&self) -> bool {
        self.core_hp() <= 0.0 || self.total_hp() <= self.max_total_hp() * 0.45
    }

    fn to_view(&self) -> BotView {
        let min_hp = self.max_total_hp() * 0.45;
        let hull_pct =
            ((self.total_hp() - min_hp) / (self.max_total_hp() - min_hp)).clamp(0.0, 1.0);
        let core_pct = self
            .modules
            .first()
            .map(|c| (c.hp / c.max_hp).clamp(0.0, 1.0))
            .unwrap_or(0.0);

        BotView {
            id: self.id,
            name: self.name.clone(),
            color: self.color.clone(),
            x: self.pos.x,
            y: self.pos.y,
            angle: self.angle,
            hp: hull_pct.min(core_pct) * 100.0,
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
    pub pos: Vec2,
    pub vel: Vec2,
    pub ttl: u32,
}

pub struct Orb {
    pub pos: Vec2,
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

        self.bots.push(BotEntity {
            id,
            name: name.to_string(),
            color: colors[id % colors.len()].to_string(),
            pos: random_pos(100.0, 900.0),
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
                pos: random_pos(200.0, 800.0),
                kind: if rand::random_bool(0.5) {
                    "heal"
                } else {
                    "overclock"
                },
            });
        }

        self.step_bots();
        self.separate_bots();
        self.step_bullets(&mut events);
        self.step_orbs(&mut events);

        for bot in &mut self.bots {
            if bot.is_dead() {
                bot.modules.iter_mut().for_each(Module::reset);
                bot.overclock_ticks = 0;
                bot.pos = random_pos(100.0, 900.0);
            }
        }

        ArenaSnapshot {
            tick: self.tick,
            bots: self.bots.iter().map(BotEntity::to_view).collect(),
            bullets: self
                .bullets
                .iter()
                .map(|b| BulletView {
                    x: b.pos.x,
                    y: b.pos.y,
                })
                .collect(),
            orbs: self
                .orbs
                .iter()
                .map(|o| OrbView {
                    x: o.pos.x,
                    y: o.pos.y,
                    kind: o.kind,
                })
                .collect(),
            events,
        }
    }

    fn step_bots(&mut self) {
        let positions: Vec<(usize, Vec2)> = self.bots.iter().map(|b| (b.id, b.pos)).collect();
        let min_b = Vec2::splat(30.0);
        let max_b = Vec2::splat(ARENA_SIZE - 30.0);

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
                .filter(|(id, _)| *id != bot.id)
                .map(|(_, p)| *p)
                .collect();

            let nearest_orb_angle = self
                .orbs
                .iter()
                .min_by(|a, b| bot.pos.distance(a.pos).total_cmp(&bot.pos.distance(b.pos)))
                .map(|o| geom::normalize_deg(geom::angle_to_deg(bot.pos, o.pos) - bot.angle))
                .unwrap_or(999.0);

            let fuel_budget = BASE_FUEL_PER_TICK
                + reactors * 3_000
                + if bot.overclock_ticks > 0 { 10_000 } else { 0 };

            let out = bot.runtime.step(
                BotInput {
                    hp: bot.core_hp(),
                    pos: bot.pos,
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
            bot.pos = (bot.pos
                + Vec2::from_angle(bot.angle.to_radians()) * out.speed * 4.5 * speed_mul)
                .clamp(min_b, max_b);

            for idx in out.fired_modules {
                let (bpos, angle) = (bot.pos, bot.angle);
                if let Some(m) = bot.modules.get_mut(idx).filter(|m| m.is_ready_cannon()) {
                    m.cooldown = 15.0;
                    let dir = Vec2::from_angle((angle + m.dir).to_radians());
                    self.bullets.push(Bullet {
                        owner_id: bot.id,
                        pos: m.world_pos(bpos, angle) + dir * 4.0,
                        vel: dir * 14.0,
                        ttl: 70,
                    });
                }
            }
        }
    }

    fn separate_bots(&mut self) {
        let min_dist = 34.0_f32;
        let min_b = Vec2::splat(30.0);
        let max_b = Vec2::splat(ARENA_SIZE - 30.0);

        for i in 0..self.bots.len() {
            for j in (i + 1)..self.bots.len() {
                let delta = self.bots[j].pos - self.bots[i].pos;
                let d = delta.length();
                if d < min_dist {
                    let n = if d > 1e-4 { delta / d } else { Vec2::X };
                    let push = n * ((min_dist - d) * 0.5);
                    self.bots[i].pos = (self.bots[i].pos - push).clamp(min_b, max_b);
                    self.bots[j].pos = (self.bots[j].pos + push).clamp(min_b, max_b);
                }
            }
        }
    }

    fn step_bullets(&mut self, events: &mut Vec<String>) {
        let mut alive = Vec::with_capacity(self.bullets.len());
        let max_b = Vec2::splat(ARENA_SIZE);

        for mut b in self.bullets.drain(..) {
            let prev = b.pos;
            b.pos += b.vel;
            if b.ttl == 0 || b.pos.cmplt(Vec2::ZERO).any() || b.pos.cmpgt(max_b).any() {
                continue;
            }
            b.ttl -= 1;

            let mut hit = None;
            for bot in &mut self.bots {
                if bot.id == b.owner_id || b.pos.distance(bot.pos) > 50.0 {
                    continue;
                }
                let a = geom::world_to_local(bot.pos, bot.angle, prev);
                let b_loc = geom::world_to_local(bot.pos, bot.angle, b.pos);

                let hit_mod = bot
                    .modules
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.is_alive())
                    .filter_map(|(idx, m)| {
                        let c = m.local_pos();
                        (geom::dist_to_segment(c, a, b_loc) <= CELL_SIZE * 0.72)
                            .then_some((idx, a.distance(c)))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1));

                if let Some((idx, _)) = hit_mod {
                    bot.modules[idx].hp -= 25.0;
                    let dead = bot.is_dead();
                    let broken = !bot.modules[idx].is_alive() && !dead;
                    hit = Some((
                        b.owner_id,
                        bot.name.clone(),
                        dead,
                        broken,
                        bot.modules[idx].kind,
                    ));
                    break;
                }
            }

            if let Some((killer_id, victim, dead, broken, kind)) = hit {
                if dead {
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
                if bot.pos.distance(orb.pos) < 28.0 {
                    if orb.kind == "heal" {
                        for m in bot.modules.iter_mut().filter(|m| m.is_alive()) {
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
