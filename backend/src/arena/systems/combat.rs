use glam::Vec2;
use hecs::{Entity, World};

use crate::arena::{
    components::{Bot, Bullet, Hull, Pos, Vel},
    config::{ARENA_SIZE, BULLET_BROADPHASE_RADIUS, BULLET_DAMAGE, MODULE_HIT_RADIUS},
    geom,
};

pub fn step_bullets(ecs: &mut World, events: &mut Vec<String>) {
    let max_b = Vec2::splat(ARENA_SIZE);
    let mut to_despawn = Vec::new();
    let mut moved_bullets = Vec::new();

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
            if bot_entity == owner || curr.distance(pos.0) > BULLET_BROADPHASE_RADIUS {
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
                    (geom::dist_to_segment(c, a, b_loc) <= MODULE_HIT_RADIUS)
                        .then_some((idx, a.distance(c)))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));

            if let Some((idx, _)) = hit_mod {
                hull.0[idx].hp -= BULLET_DAMAGE;
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
