use glam::Vec2;
use hecs::{Entity, World};

use super::random_pos;
use crate::arena::{
    components::{Bot, Hull, Orb, Pos},
    config::{
        ARENA_SIZE, MAX_ORBS, ORB_HEAL_RATIO, ORB_PICKUP_RADIUS, ORB_SPAWN_INTERVAL,
        ORB_SPAWN_MARGIN, OVERCLOCK_TICKS,
    },
};

pub fn spawn_orbs(ecs: &mut World, tick: u64) {
    if !tick.is_multiple_of(ORB_SPAWN_INTERVAL) {
        return;
    }
    if ecs.query::<&Orb>().iter().count() < MAX_ORBS {
        ecs.spawn((
            Pos(random_pos(ORB_SPAWN_MARGIN, ARENA_SIZE - ORB_SPAWN_MARGIN)),
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

pub fn step_orbs(ecs: &mut World, events: &mut Vec<String>) {
    let orbs: Vec<(Entity, Vec2, &'static str)> = ecs
        .query::<(Entity, &Pos, &Orb)>()
        .iter()
        .map(|(e, p, o)| (e, p.0, o.kind))
        .collect();

    let mut picked = Vec::new();
    for (orb_e, orb_pos, kind) in orbs {
        for (pos, bot, hull) in ecs.query_mut::<(&Pos, &mut Bot, &mut Hull)>() {
            if pos.0.distance(orb_pos) < ORB_PICKUP_RADIUS {
                if kind == "heal" {
                    for m in hull.0.iter_mut().filter(|m| m.is_alive()) {
                        m.hp = (m.hp + m.max_hp * ORB_HEAL_RATIO).min(m.max_hp);
                    }
                } else {
                    bot.overclock_ticks = OVERCLOCK_TICKS;
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
