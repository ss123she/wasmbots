use glam::Vec2;

pub fn normalize_deg(a: f32) -> f32 {
    (a + 180.0).rem_euclid(360.0) - 180.0
}

pub fn angle_to_deg(from: Vec2, to: Vec2) -> f32 {
    let d = to - from;
    d.y.atan2(d.x).to_degrees()
}

pub fn in_cone(
    origin: Vec2,
    bot_angle: f32,
    offset_deg: f32,
    fov_deg: f32,
    target: Vec2,
) -> Option<f32> {
    let dir = angle_to_deg(origin, target);
    let scan_dir = bot_angle + offset_deg;
    (normalize_deg(dir - scan_dir).abs() <= fov_deg * 0.5).then(|| origin.distance(target))
}

pub fn local_to_world(origin: Vec2, angle_deg: f32, local: Vec2) -> Vec2 {
    origin + Vec2::from_angle(angle_deg.to_radians()).rotate(local)
}

pub fn world_to_local(origin: Vec2, angle_deg: f32, world: Vec2) -> Vec2 {
    Vec2::from_angle((-angle_deg).to_radians()).rotate(world - origin)
}

pub fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq <= 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}
