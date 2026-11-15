use glam::Vec2;
use hecs::Entity;

use crate::{arena::modules::Module, wasm::sandbox::BotRuntime};

pub struct Pos(pub Vec2);
pub struct Vel(pub Vec2);

pub struct Bot {
    pub name: String,
    pub color: String,
    pub angle: f32,
    pub kills: u32,
    pub overclock_ticks: u32,
    pub runtime: BotRuntime,
    pub last_scans: Vec<(f32, f32)>,
}

pub struct Hull(pub Vec<Module>);

impl Hull {
    pub fn core_hp(&self) -> f32 {
        self.0.first().map(|m| m.hp).unwrap_or(0.0)
    }

    pub fn total_hp(&self) -> f32 {
        self.0.iter().map(|m| m.hp.max(0.0)).sum()
    }

    pub fn max_total_hp(&self) -> f32 {
        self.0.iter().map(|m| m.max_hp).sum()
    }

    pub fn is_dead(&self) -> bool {
        self.core_hp() <= 0.0 || self.total_hp() <= self.max_total_hp() * 0.45
    }

    pub fn hp_display(&self) -> f32 {
        let min_hp = self.max_total_hp() * 0.45;
        let hull_pct =
            ((self.total_hp() - min_hp) / (self.max_total_hp() - min_hp)).clamp(0.0, 1.0);
        let core_pct = self
            .0
            .first()
            .map(|c| (c.hp / c.max_hp).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        hull_pct.min(core_pct) * 100.0
    }
}

pub struct Bullet {
    pub owner: Entity,
    pub ttl: u32,
}

pub struct Orb {
    pub kind: &'static str,
}
