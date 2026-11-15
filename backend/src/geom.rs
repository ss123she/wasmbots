pub fn normalize_deg(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

pub fn angle_to_deg(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    (y2 - y1).atan2(x2 - x1).to_degrees()
}

pub fn dist(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

pub fn in_cone(
    bx: f32,
    by: f32,
    bot_angle: f32,
    offset_deg: f32,
    fov_deg: f32,
    tx: f32,
    ty: f32,
) -> Option<f32> {
    let dir_to_target = angle_to_deg(bx, by, tx, ty);
    let scan_dir = bot_angle + offset_deg;
    if normalize_deg(dir_to_target - scan_dir).abs() <= fov_deg * 0.5 {
        Some(dist(bx, by, tx, ty))
    } else {
        None
    }
}

pub fn local_to_world(bx: f32, by: f32, angle_deg: f32, lx: f32, ly: f32) -> (f32, f32) {
    let (s, c) = angle_deg.to_radians().sin_cos();
    (bx + lx * c - ly * s, by + lx * s + ly * c)
}

pub fn world_to_local(bx: f32, by: f32, angle_deg: f32, wx: f32, wy: f32) -> (f32, f32) {
    let (dx, dy) = (wx - bx, wy - by);
    let (s, c) = (-angle_deg).to_radians().sin_cos();
    (dx * c - dy * s, dx * s + dy * c)
}
