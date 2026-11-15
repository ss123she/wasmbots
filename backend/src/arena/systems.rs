use glam::Vec2;
use hecs::{Entity, World};

use crate::{
    arena::{
        components::{Bot, Bullet, Hull, Orb, Pos, Vel},
        geom,
        modules::{CELL_SIZE, Module, ModuleKind, compute_mobility, count_alive, min_cannon_cd},
        snapshot::{ArenaSnapshot, BotView, BulletView, OrbView},
    },
    wasm::sandbox::{BASE_FUEL_PER_TICK, BotInput},
};

pub const ARENA_SIZE: f32 = 1000.0;

pub fn random_pos(min: f32, max: f32) -> Vec2 {
    Vec2::new(rand::random_range(min..max), rand::random_range(min..max))
}

pub fn spawn_orbs(ecs: &mut World, tick: u64) {
    if tick % 120 != 0 {
        return;
    }
    let orb_count = ecs.query::<&Orb>().iter().count();
    if orb_count < 3 {
        ecs.spawn((
            Pos(random_pos(200.0, 800.0)),
            Orb {
                kind: if rand::random_bool(0.5) {
                    "heal"
                } else {
                    "overclock"
                },
            },
        ));
    }
}

pub fn step_bots(ecs: &mut World) {
    let bot_positions: Vec<(Entity, Vec2)> = ecs
        .query::<(Entity, &Pos, &Bot)>()
        .iter()
        .map(|(e, p, _)| (e, p.0))
        .collect();

    let orb_positions: Vec<Vec2> = ecs
        .query::<(&Pos, &Orb)>()
        .iter()
        .map(|(p, _)| p.0)
        .collect();

    let min_b = Vec2::splat(30.0);
    let max_b = Vec2::splat(ARENA_SIZE - 30.0);
    let mut new_bullets: Vec<(Entity, Vec2, Vec2)> = Vec::new();

    for (entity, pos, bot, hull) in ecs.query_mut::<(Entity, &mut Pos, &mut Bot, &mut Hull)>() {
        bot.overclock_ticks = bot.overclock_ticks.saturating_sub(1);
        let reactors = count_alive(&hull.0, ModuleKind::Reactor) as u64;
        let cd_step =
            1.0 + (reactors as f32 * 0.15) + if bot.overclock_ticks > 0 { 0.8 } else { 0.0 };

        for m in &mut hull.0 {
            m.cooldown = (m.cooldown - cd_step).max(0.0);
        }

        let enemies = bot_positions
            .iter()
            .filter(|(e, _)| *e != entity)
            .map(|(_, p)| *p)
            .collect();

        let nearest_orb_angle = orb_positions
            .iter()
            .min_by(|a, b| pos.0.distance(**a).total_cmp(&pos.0.distance(**b)))
            .map(|&o| geom::normalize_deg(geom::angle_to_deg(pos.0, o) - bot.angle))
            .unwrap_or(999.0);

        let fuel_budget = BASE_FUEL_PER_TICK
            + reactors * 3_000
            + if bot.overclock_ticks > 0 { 10_000 } else { 0 };

        let out = bot.runtime.step(
            BotInput {
                hp: hull.core_hp(),
                pos: pos.0,
                angle: bot.angle,
                cooldown: min_cannon_cd(&hull.0),
                nearest_orb_angle,
                enemies,
                modules: hull.0.clone(),
            },
            fuel_budget,
        );

        let (speed_mul, turn_mul) = compute_mobility(&hull.0);
        bot.last_scans = out.scans;
        bot.angle = geom::normalize_deg(bot.angle + out.turn * 5.0 * turn_mul);
        pos.0 = (pos.0 + Vec2::from_angle(bot.angle.to_radians()) * out.speed * 4.5 * speed_mul)
            .clamp(min_b, max_b);

        for idx in out.fired_modules {
            if let Some(m) = hull.0.get_mut(idx).filter(|m| m.is_ready_cannon()) {
                m.cooldown = 15.0;
                let dir = Vec2::from_angle((bot.angle + m.dir).to_radians());
                new_bullets.push((
                    entity,
                    m.world_pos(pos.0, bot.angle) + dir * 4.0,
                    dir * 14.0,
                ));
            }
        }
    }

    for (owner, pos, vel) in new_bullets {
        ecs.spawn((Pos(pos), Vel(vel), Bullet { owner, ttl: 70 }));
    }
}

pub fn separate_bots(ecs: &mut World) {
    let mut bots: Vec<(Entity, Vec2)> = ecs
        .query::<(Entity, &Pos, &Bot)>()
        .iter()
        .map(|(e, p, _)| (e, p.0))
        .collect();

    let min_dist = 34.0_f32;
    let min_b = Vec2::splat(30.0);
    let max_b = Vec2::splat(ARENA_SIZE - 30.0);

    for i in 0..bots.len() {
        for j in (i + 1)..bots.len() {
            let delta = bots[j].1 - bots[i].1;
            let d = delta.length();
            if d < min_dist {
                let n = if d > 1e-4 { delta / d } else { Vec2::X };
                let push = n * ((min_dist - d) * 0.5);
                bots[i].1 = (bots[i].1 - push).clamp(min_b, max_b);
                bots[j].1 = (bots[j].1 + push).clamp(min_b, max_b);
            }
        }
    }

    for (e, pos) in bots {
        if let Ok(mut p) = ecs.get::<&mut Pos>(e) {
            p.0 = pos;
        }
    }
}

pub fn step_bullets(ecs: &mut World, events: &mut Vec<String>) {
    let max_b = Vec2::splat(ARENA_SIZE);
    let mut to_despawn: Vec<Entity> = Vec::new();
    let mut moved_bullets: Vec<(Entity, Entity, Vec2, Vec2)> = Vec::new();

    for (e, pos, vel, b) in ecs.query_mut::<(Entity, &mut Pos, &Vel, &mut Bullet)>() {
        let prev = pos.0;
        pos.0 += vel.0;
        if b.ttl == 0 || pos.0.cmplt(Vec2::ZERO).any() || pos.0.cmpgt(max_b).any() {
            to_despawn.push(e);
            continue;
        }
        b.ttl -= 1;
        moved_bullets.push((e, b.owner, prev, pos.0));
    }

    let mut kills = Vec::new();
    for (b_entity, owner, prev, curr) in moved_bullets {
        for (bot_entity, pos, bot, hull) in ecs.query_mut::<(Entity, &Pos, &Bot, &mut Hull)>() {
            if bot_entity == owner || curr.distance(pos.0) > 50.0 {
                continue;
            }
            let a = geom::world_to_local(pos.0, bot.angle, prev);
            let b_loc = geom::world_to_local(pos.0, bot.angle, curr);

            let hit_mod = hull
                .0
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
                hull.0[idx].hp -= 25.0;
                if hull.is_dead() {
                    kills.push((owner, bot.name.clone()));
                } else if !hull.0[idx].is_alive() {
                    events.push(format!("{} lost {:?}", bot.name, hull.0[idx].kind));
                }
                to_despawn.push(b_entity);
                break;
            }
        }
    }

    for (killer, victim) in kills {
        if let Ok(mut k) = ecs.get::<&mut Bot>(killer) {
            k.kills += 1;
            events.push(format!("{} destroyed {}", k.name, victim));
        }
    }

    for e in to_despawn {
        let _ = ecs.despawn(e);
    }
}

pub fn step_orbs(ecs: &mut World, events: &mut Vec<String>) {
    let orbs: Vec<(Entity, Vec2, &'static str)> = ecs
        .query::<(Entity, &Pos, &Orb)>()
        .iter()
        .map(|(e, p, o)| (e, p.0, o.kind))
        .collect();

    let mut picked = Vec::new();
    for (orb_e, orb_pos, kind) in orbs {
        for (pos, bot, hull) in ecs.query_mut::<(&Pos, &mut Bot, &mut Hull)>() {
            if pos.0.distance(orb_pos) < 28.0 {
                if kind == "heal" {
                    for m in hull.0.iter_mut().filter(|m| m.is_alive()) {
                        m.hp = (m.hp + m.max_hp * 0.45).min(m.max_hp);
                    }
                } else {
                    bot.overclock_ticks = 180;
                }
                events.push(format!("{} picked {}", bot.name, kind));
                picked.push(orb_e);
                break;
            }
        }
    }

    for e in picked {
        let _ = ecs.despawn(e);
    }
}

pub fn respawn_dead(ecs: &mut World) {
    for (pos, bot, hull) in ecs.query_mut::<(&mut Pos, &mut Bot, &mut Hull)>() {
        if hull.is_dead() {
            hull.0.iter_mut().for_each(Module::reset);
            bot.overclock_ticks = 0;
            pos.0 = random_pos(100.0, 900.0);
        }
    }
}

pub fn build_snapshot(ecs: &World, tick: u64, events: Vec<String>) -> ArenaSnapshot {
    let bots = ecs
        .query::<(Entity, &Pos, &Bot, &Hull)>()
        .iter()
        .map(|(e, pos, b, hull)| BotView {
            id: e.id() as usize,
            name: b.name.clone(),
            color: b.color.clone(),
            x: pos.0.x,
            y: pos.0.y,
            angle: b.angle,
            hp: hull.hp_display(),
            kills: b.kills,
            cpu_pct: (b.runtime.last_fuel_used as f32 / b.runtime.last_fuel_budget as f32) * 100.0,
            overheated: b.runtime.overheated_ticks > 0,
            scans: b.last_scans.clone(),
            modules: hull.0.iter().map(Module::to_view).collect(),
        })
        .collect();

    let bullets = ecs
        .query::<(&Pos, &Bullet)>()
        .iter()
        .map(|(p, _)| BulletView { x: p.0.x, y: p.0.y })
        .collect();

    let orbs = ecs
        .query::<(&Pos, &Orb)>()
        .iter()
        .map(|(p, o)| OrbView {
            x: p.0.x,
            y: p.0.y,
            kind: o.kind,
        })
        .collect();

    ArenaSnapshot {
        tick,
        bots,
        bullets,
        orbs,
        events,
    }
}
