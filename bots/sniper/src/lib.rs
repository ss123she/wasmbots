use std::sync::atomic::{AtomicI32, Ordering};

const ARMOR: i32 = 1;
const THRUSTER: i32 = 2;
const CANNON: i32 = 3;
const RADAR: i32 = 4;
const REACTOR: i32 = 5;

static CANNON_ID: AtomicI32 = AtomicI32::new(-1);
static RADAR_ID: AtomicI32 = AtomicI32::new(-1);
static SWEEP_STEP: AtomicI32 = AtomicI32::new(0);

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn place_module(gx: i32, gy: i32, kind: i32, dir_deg: f32) -> i32;
    fn get_self(stat: i32) -> f32;
    fn module_hp(id: i32) -> f32;
    fn drive(speed: f32, turn: f32);
    fn fire() -> i32;
    fn fire_module(id: i32) -> i32;
    fn scan_enemy(offset_deg: f32, fov_deg: f32) -> f32;
    fn scan_from(id: i32, offset_deg: f32, fov_deg: f32) -> f32;
    fn scan_orb() -> f32;
}

fn norm_deg(mut a: f32) -> f32 {
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

unsafe fn smart_scan(offset_deg: f32, fov_deg: f32) -> f32 {
    let rid = RADAR_ID.load(Ordering::Relaxed);
    unsafe {
        if rid > 0 && module_hp(rid) > 0.0 {
            scan_from(rid, offset_deg, fov_deg)
        } else {
            scan_enemy(offset_deg, fov_deg)
        }
    }
}

unsafe fn smart_fire() {
    let cid = CANNON_ID.load(Ordering::Relaxed);
    unsafe {
        if cid > 0 && module_hp(cid) > 0.0 {
            fire_module(cid);
        } else {
            fire();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn build() {
    unsafe {
        place_module(1, 0, ARMOR, 0.0);
        let cannon = place_module(2, 0, CANNON, 0.0);
        let radar = place_module(0, 1, RADAR, 0.0);
        place_module(0, -1, REACTOR, 0.0);
        place_module(-1, 0, THRUSTER, 180.0);
        place_module(-1, -1, THRUSTER, 180.0);

        CANNON_ID.store(cannon, Ordering::Relaxed);
        RADAR_ID.store(radar, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tick() {
    unsafe {
        let hp = get_self(0);
        let x = get_self(1);
        let y = get_self(2);
        let angle = get_self(3);

        let front = smart_scan(0.0, 14.0);
        if front > 0.0 {
            if front < 750.0 {
                smart_fire();
            }
            if front < 250.0 {
                drive(1.0, 0.85);
            } else if front > 440.0 {
                drive(0.7, 0.0);
            } else {
                drive(0.2, 0.0);
            }
            return;
        }

        if !(110.0..=890.0).contains(&x) || !(110.0..=890.0).contains(&y) {
            let to_center = (500.0 - y).atan2(500.0 - x).to_degrees();
            let turn = (norm_deg(to_center - angle) / 35.0).clamp(-1.0, 1.0);
            drive(0.9, turn);
            return;
        }

        if smart_scan(-24.0, 32.0) > 0.0 {
            drive(0.55, -0.75);
            return;
        }
        if smart_scan(24.0, 32.0) > 0.0 {
            drive(0.55, 0.75);
            return;
        }

        let orb = scan_orb();
        if hp < 32.0 && orb < 180.0 {
            drive(1.0, (orb / 40.0).clamp(-1.0, 1.0));
        } else {
            let step = SWEEP_STEP.fetch_add(1, Ordering::Relaxed) % 4;
            let sweep_angle = match step {
                0 => -75.0,
                1 => 75.0,
                2 => -135.0,
                _ => 135.0,
            };
            let spotted = smart_scan(sweep_angle, 55.0);
            if spotted > 0.0 {
                drive(0.5, sweep_angle.signum());
            } else {
                drive(0.45, 0.35);
            }
        }
    }
}
