use glam::Vec2;
use hecs::{Entity, World};

use super::random_spawn;
use crate::{
    arena::{
        components::{Bot, Bullet, Hull, Orb, Pos, Vel},
        config::{
            ARENA_SIZE, BASE_FUEL_PER_TICK, BOT_BASE_SPEED, BOT_BASE_TURN, BOT_SEPARATION_DIST,
            BOT_WALL_MARGIN, BULLET_SPAWN_OFFSET, BULLET_SPEED, BULLET_TTL, CANNON_COOLDOWN,
            OVERCLOCK_FUEL_BONUS, REACTOR_FUEL_BONUS,
        },
        geom,
        modules::{Module, ModuleKind, compute_mobility, count_alive, min_cannon_cd},
    },
    wasm::sandbox::BotInput,
};

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

    let min_b = Vec2::splat(BOT_WALL_MARGIN);
    let max_b = Vec2::splat(ARENA_SIZE - BOT_WALL_MARGIN);
    let mut new_bullets = Vec::new();

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
            + reactors * REACTOR_FUEL_BONUS
            + if bot.overclock_ticks > 0 {
                OVERCLOCK_FUEL_BONUS
            } else {
                0
            };

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
        bot.angle = geom::normalize_deg(bot.angle + out.turn * BOT_BASE_TURN * turn_mul);
        pos.0 = (pos.0
            + Vec2::from_angle(bot.angle.to_radians()) * out.speed * BOT_BASE_SPEED * speed_mul)
            .clamp(min_b, max_b);

        for idx in out.fired_modules {
            if let Some(m) = hull.0.get_mut(idx).filter(|m| m.is_ready_cannon()) {
                m.cooldown = CANNON_COOLDOWN;
                let dir = Vec2::from_angle((bot.angle + m.dir).to_radians());
                new_bullets.push((
                    entity,
                    m.world_pos(pos.0, bot.angle) + dir * BULLET_SPAWN_OFFSET,
                    dir * BULLET_SPEED,
                ));
            }
        }
    }

    for (owner, pos, vel) in new_bullets {
        ecs.spawn((
            Pos(pos),
            Vel(vel),
            Bullet {
                owner,
                ttl: BULLET_TTL,
            },
        ));
    }
}

pub fn separate_bots(ecs: &mut World) {
    let mut bots: Vec<(Entity, Vec2)> = ecs
        .query::<(Entity, &Pos, &Bot)>()
        .iter()
        .map(|(e, p, _)| (e, p.0))
        .collect();

    let min_b = Vec2::splat(BOT_WALL_MARGIN);
    let max_b = Vec2::splat(ARENA_SIZE - BOT_WALL_MARGIN);

    for i in 0..bots.len() {
        for j in (i + 1)..bots.len() {
            let delta = bots[j].1 - bots[i].1;
            let d = delta.length();
            if d < BOT_SEPARATION_DIST {
                let n = if d > 1e-4 { delta / d } else { Vec2::X };
                let push = n * ((BOT_SEPARATION_DIST - d) * 0.5);
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

pub fn respawn_dead(ecs: &mut World) {
    for (pos, bot, hull) in ecs.query_mut::<(&mut Pos, &mut Bot, &mut Hull)>() {
        if hull.is_dead() {
            hull.0.iter_mut().for_each(Module::reset);
            bot.overclock_ticks = 0;
            pos.0 = random_spawn();
        }
    }
}
