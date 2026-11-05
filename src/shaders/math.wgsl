struct Scene {
    view: mat4x4f,
    time: f32,
}

fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + vec3f(dot(p3, vec3f(p3.y, p3.z, p3.x)) + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + vec2f(1.0, 0.0));
    let c = hash12(i + vec2f(0.0, 1.0));
    let d = hash12(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm4(p0: vec2f) -> f32 {
    var p = p0;
    var sum = 0.0;
    var amp = 0.5;
    var norm = 0.0;
    for(var i = 0; i < 4; i = i + 1) {
        sum = sum + vnoise(p) * amp;
        norm = norm + amp;
        amp = amp * 0.5;
        p = p * 2.03 + vec2f(11.7, -7.3);
    }
    return sum / norm;
}
