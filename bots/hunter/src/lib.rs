#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn drive(speed: f32, turn: f32);
    fn fire() -> i32;
    fn scan_enemy(offset_deg: f32, fov_deg: f32) -> f32;
    fn scan_orb() -> f32;
}

#[unsafe(no_mangle)]
pub extern "C" fn tick() {
    unsafe {
        let dist = scan_enemy(0.0, 25.0);
        if dist > 0.0 && dist < 500.0 {
            fire();
            drive(0.9, 0.0);
        } else {
            let orb = scan_orb();
            if orb < 180.0 {
                drive(1.0, (orb / 40.0).clamp(-1.0, 1.0));
            } else {
                drive(0.7, 0.45);
            }
        }
    }
}
