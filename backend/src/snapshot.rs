use serde::Serialize;

#[derive(Serialize)]
pub struct ModuleView {
    pub gx: i32,
    pub gy: i32,
    pub kind: u8,
    pub dir: f32,
    pub hp_pct: f32,
}

#[derive(Serialize)]
pub struct BotView {
    pub id: usize,
    pub name: String,
    pub color: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub hp: f32,
    pub kills: u32,
    pub cpu_pct: f32,
    pub overheated: bool,
    pub scans: Vec<(f32, f32)>,
    pub modules: Vec<ModuleView>,
}

#[derive(Serialize)]
pub struct BulletView {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize)]
pub struct OrbView {
    pub x: f32,
    pub y: f32,
    pub kind: &'static str,
}

#[derive(Serialize)]
pub struct ArenaSnapshot {
    pub tick: u64,
    pub bots: Vec<BotView>,
    pub bullets: Vec<BulletView>,
    pub orbs: Vec<OrbView>,
    pub events: Vec<String>,
}
