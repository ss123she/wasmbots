use glam::Vec2;

use super::{
    config::{ARENA_SIZE, BOT_SPAWN_MARGIN},
    snapshot::ArenaSnapshot,
};

mod bots;
mod combat;
mod orbs;
mod snapshot;

pub fn random_pos(min: f32, max: f32) -> Vec2 {
    Vec2::new(rand::random_range(min..max), rand::random_range(min..max))
}

pub fn random_spawn() -> Vec2 {
    random_pos(BOT_SPAWN_MARGIN, ARENA_SIZE - BOT_SPAWN_MARGIN)
}

pub fn tick(ecs: &mut hecs::World, tick: u64) -> ArenaSnapshot {
    let mut events = Vec::new();

    orbs::spawn_orbs(ecs, tick);
    bots::step_bots(ecs);
    bots::separate_bots(ecs);
    combat::step_bullets(ecs, &mut events);
    orbs::step_orbs(ecs, &mut events);
    bots::respawn_dead(ecs);

    snapshot::build_snapshot(ecs, tick, events)
}
