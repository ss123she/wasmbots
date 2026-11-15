use crate::geom;

pub const GRID_RADIUS: i32 = 2;
pub const CELL_SIZE: f32 = 10.0;
pub const STARTING_CREDITS: i32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ModuleKind {
    Core = 0,
    Armor = 1,
    Thruster = 2,
    Cannon = 3,
    Radar = 4,
    Reactor = 5,
}

impl ModuleKind {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Core),
            1 => Some(Self::Armor),
            2 => Some(Self::Thruster),
            3 => Some(Self::Cannon),
            4 => Some(Self::Radar),
            5 => Some(Self::Reactor),
            _ => None,
        }
    }

    pub fn stats(self) -> (i32, f32, f32) {
        // (cost, max_hp, mass)
        match self {
            Self::Core => (0, 60.0, 2.0),
            Self::Armor => (10, 90.0, 2.5),
            ModuleKind::Thruster => (20, 25.0, 1.0),
            ModuleKind::Cannon => (30, 30.0, 1.5),
            ModuleKind::Radar => (15, 20.0, 0.8),
            ModuleKind::Reactor => (25, 25.0, 1.5),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModuleSpec {
    pub gx: i32,
    pub gy: i32,
    pub kind: ModuleKind,
    pub dir: f32,
    pub max_hp: f32,
    pub mass: f32,
}

pub struct ModuleState {
    pub hp: f32,
    pub cooldown: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ModuleSnapshot {
    pub gx: f32,
    pub gy: f32,
    pub kind: ModuleKind,
    pub dir: f32,
    pub hp: f32,
    pub cooldown: f32,
}

pub fn core_spec() -> ModuleSpec {
    let (_, max_hp, mass) = ModuleKind::Core.stats();
    ModuleSpec {
        gx: 0,
        gy: 0,
        kind: ModuleKind::Core,
        dir: 0.0,
        max_hp,
        mass,
    }
}

pub fn default_blueprint() -> Vec<ModuleSpec> {
    [
        (0, 0, ModuleKind::Core, 0.0),
        (-1, 0, ModuleKind::Thruster, 180.0),
        (1, 0, ModuleKind::Cannon, 0.0),
        (0, 1, ModuleKind::Armor, 0.0),
        (0, -1, ModuleKind::Armor, 0.0),
        (-1, 1, ModuleKind::Radar, 0.0),
    ]
    .into_iter()
    .map(|(gx, gy, kind, dir)| {
        let (_, max_hp, mass) = kind.stats();
        ModuleSpec {
            gx,
            gy,
            kind,
            dir,
            max_hp,
            mass,
        }
    })
    .collect()
}

pub fn try_place(
    blueprint: &mut Vec<ModuleSpec>,
    credits: &mut i32,
    gx: i32,
    gy: i32,
    kind_i: i32,
    dir: f32,
) -> i32 {
    if gx.abs() > GRID_RADIUS || gy.abs() > GRID_RADIUS {
        return -2;
    }
    let Some(kind) = ModuleKind::from_i32(kind_i) else {
        return -3;
    };
    if kind == ModuleKind::Core {
        return -3;
    }
    if blueprint.iter().any(|m| m.gx == gx && m.gy == gy) {
        return -4;
    }
    let connected = blueprint
        .iter()
        .any(|m| (m.gx - gx).abs() + (m.gy - gy).abs() == 1);
    if !connected {
        return -5;
    }

    let (cost, max_hp, mass) = kind.stats();
    if *credits < cost {
        return -6;
    }

    *credits -= cost;
    let id = blueprint.len() as i32;
    blueprint.push(ModuleSpec {
        gx,
        gy,
        kind,
        dir: geom::normalize_deg(dir),
        max_hp,
        mass,
    });
    id
}

pub fn compute_mobility(blueprint: &[ModuleSpec], states: &[ModuleState]) -> (f32, f32) {
    let mut total_mass = 0.0_f32;
    let mut thrust = 0.5_f32;
    let mut turn_power = 0.7_f32;

    for (spec, st) in blueprint.iter().zip(states) {
        if st.hp <= 0.0 {
            continue;
        }
        total_mass += spec.mass;
        if spec.kind == ModuleKind::Thruster {
            thrust += 1.4;
            let lever = ((spec.gx.pow(2) + spec.gy.pow(2)) as f32).sqrt().max(1.0);
            turn_power += 0.45 * lever;
        }
    }

    let mass_factor = (total_mass / 6.0).sqrt().max(0.75);
    (
        (thrust / mass_factor).clamp(0.35, 2.0),
        (turn_power / mass_factor).clamp(0.45, 2.0),
    )
}
