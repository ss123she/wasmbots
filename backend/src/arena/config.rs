pub const ARENA_SIZE: f32 = 1000.0;
pub const BOT_SPAWN_MARGIN: f32 = 100.0;
pub const BOT_WALL_MARGIN: f32 = 30.0;
pub const BOT_SEPARATION_DIST: f32 = 34.0;

pub const GRID_RADIUS: i32 = 2;
pub const CELL_SIZE: f32 = 10.0;
pub const STARTING_CREDITS: i32 = 120;
pub const HULL_DEATH_RATIO: f32 = 0.45;

pub const BASE_FUEL_PER_TICK: u64 = 15_000;
pub const REACTOR_FUEL_BONUS: u64 = 3_000;
pub const OVERCLOCK_FUEL_BONUS: u64 = 10_000;

pub const BOT_BASE_SPEED: f32 = 4.5;
pub const BOT_BASE_TURN: f32 = 5.0;

pub const CANNON_COOLDOWN: f32 = 15.0;
pub const BULLET_SPEED: f32 = 14.0;
pub const BULLET_DAMAGE: f32 = 25.0;
pub const BULLET_TTL: u32 = 70;
pub const BULLET_SPAWN_OFFSET: f32 = 4.0;
pub const BULLET_BROADPHASE_RADIUS: f32 = 50.0;
pub const MODULE_HIT_RADIUS: f32 = CELL_SIZE * 0.72;

pub const RADAR_RANGE: f32 = 1500.0;
pub const CORE_SCAN_RANGE: f32 = 250.0;

pub const ORB_SPAWN_INTERVAL: u64 = 120;
pub const ORB_SPAWN_MARGIN: f32 = 200.0;
pub const MAX_ORBS: usize = 3;
pub const ORB_PICKUP_RADIUS: f32 = 28.0;
pub const ORB_HEAL_RATIO: f32 = 0.45;
pub const OVERCLOCK_TICKS: u32 = 180;
