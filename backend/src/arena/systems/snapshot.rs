use hecs::{Entity, World};

use crate::arena::{
    components::{Bot, Bullet, Hull, Orb, Pos},
    modules::Module,
    snapshot::{ArenaSnapshot, BotView, BulletView, OrbView},
};

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
