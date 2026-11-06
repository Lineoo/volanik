struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) ndc: vec2f,
}

@group(0) @binding(0) var<uniform> scene: Scene;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let pose = vec2f(f32(index / 2), f32(index % 2));

    var ret: VertexOutput;
    ret.pos = vec4f(pose * 4.0 - 1.0, 0.0, 1.0);
    ret.ndc = ret.pos.xy;
    return ret;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // 3600x fast forward
    const DAY_CYCLE = 24 * 60 * 60 / 3600;

    let pw_near = scene.view * vec4f(in.ndc, 0.0, 1.0);
    let pw_far = scene.view * vec4f(in.ndc, 1.0, 1.0);
    let p_near = pw_near.xyz / pw_near.w;
    let p_far = pw_far.xyz / pw_far.w;
    let dir = normalize(p_far - p_near);

    let rad_sun = scene.time / DAY_CYCLE * TAU;

    let ground = clamp(fs_ground(p_near, dir), vec4f(0), vec4f(1));
    let clouds = clamp(fs_clouds(p_near, dir), vec4f(0), vec4f(1));
    let sky = clamp(fs_sky(p_near, dir, vec3f(cos(rad_sun), 0, sin(rad_sun))), vec4f(0), vec4f(1));
    return clouds + (ground + sky * (1 - ground.a)) * (1 - clouds.a);
}

// Rayleigh scatter
fn fs_sky(p_near: vec3f, dir_view: vec3f, dir_sun: vec3f) -> vec4f {
    const ATMOS_RADIUS = 100e3;
    const IRRADIATION_RATE = vec3f(2);
    const BETA_RAY = vec3f(5.5e-6, 13.0e-6, 22.4e-6);
    const H_RAY = 8000;
    const MARCHING_STEPS = 16;
    const LIGHT_STEPS = 8;

    var total_rayleigh = vec3f();
    let dir_cos = dot(dir_view, dir_sun);
    let phase_r = 3.0 / (16.0 * PI) * (1 + dir_cos * dir_cos);

    var od_view = 0.0;
    let ds_view = spherical_intersect(p_near, dir_view, ATMOS_RADIUS) / MARCHING_STEPS;
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let p_view = p_near + dir_view * ds_view * f32(i);
        od_view += exp(-p_view.z / H_RAY) * ds_view;

        var od_sun = 0.0;
        let ds_sun = spherical_intersect(p_view, dir_sun, ATMOS_RADIUS) / LIGHT_STEPS;
        for (var j = 0; j < LIGHT_STEPS; j++) {
            let p_sun = p_view + dir_sun * ds_sun * f32(j);
            od_sun += exp(-p_sun.z / H_RAY) * ds_sun;
        }

        let scatter = BETA_RAY * phase_r * exp(-p_view.z / H_RAY);
        let transmission = exp(-BETA_RAY * od_view) * exp(-BETA_RAY * od_sun);
        total_rayleigh += transmission * scatter * IRRADIATION_RATE * ds_view;
    }

    return vec4f(total_rayleigh, 1.0);
}

fn fs_clouds(p: vec3f, dir: vec3f) -> vec4f {
    const CLOUD_LOW_HEIGHT = 2e3;
    const CLOUD_HIGH_HEIGHT = 5e3;
    const MARCHING_STEPS = 32;

    let s_low = spherical_intersect_dual(p, dir, CLOUD_LOW_HEIGHT);
    let s_high = spherical_intersect_dual(p, dir, CLOUD_HIGH_HEIGHT);
    let s_far = min(max(s_low.x, s_high.x), 1e5);
    let s_near = max(min(s_low.x, s_high.x), 0);

    var sum = 0.0;
    var s = 0.0;
    let p_near = p + dir * s_near; 
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let noise = fbm4_vnoise3((p_near + dir * s) * vec3f(6e-4, 6e-4, 3e-4) + vec3f(0, scene.time * 1e-1, 0));
        let density = smoothstep(0.65, 0.75, noise);
        let step = smoothstep(0.35, 0.75, noise);
        let ds = min((s_far - s_near) / MARCHING_STEPS, 1e4 * exp(-4 * step));
        sum += density * ds;
        s += ds;
    }

    return vec4f(1 - exp(-1e-4 * sum));
}

fn fs_ground(p: vec3f, dir: vec3f) -> vec4f {
    const GROUND_HEIGHT = -10;

    let s = spherical_intersect(p, dir, GROUND_HEIGHT);
    let p_ground = p + dir * s;
    let color = mix(vec4(0.4, 0.6, 0.2, 1.0), vec4(0.3, 0.3, 0.1, 1.0), fbm4_vnoise3(p_ground / 100));
    return select(vec4f(), color, s > 0);
}

struct Intersect {
    s: f32,
    f: bool,
}

const PLANET_RADIUS = 6371e0;

// r^2 = a^2 + b^2 - 2abcos
// b^2 - (2acos) b + (a^2 - r^2) = 0
// delta = 4(acos)^2 - 4 (a^2 - r^2)
// t = (- 2acos +/- sqrt(delta)) / 2
// t = - acos +/- sqrt(delta / 4)
//
// precise loss: PLANET_RADIUS (big)
fn spherical_intersect(p: vec3f, d: vec3f, w: f32) -> f32 {
    let a_cos = fma(PLANET_RADIUS, d.z, dot(p, d));
    let c = fma(p.z - w, p.z + w + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    let frac_d4 = fma(a_cos, a_cos, -c);
    if (frac_d4 < 0.0) { return 0.0; }
    let sq = sqrt(frac_d4);
    let t0 = -a_cos - sq;
    let t1 = -a_cos + sq;
    return select(min(t0, t1), t1, t0 < 0);
}

fn spherical_intersect_dual(p: vec3f, d: vec3f, w: f32) -> vec2f {
    let a_cos = fma(PLANET_RADIUS, d.z, dot(p, d));
    let c = fma(p.z - w, p.z + w + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    let frac_d4 = fma(a_cos, a_cos, -c);
    if (frac_d4 < 0.0) { return 0.0; }
    let sq = sqrt(frac_d4);
    let t0 = -a_cos - sq;
    let t1 = -a_cos + sq;
    return vec2f(select(min(t0, t1), t1, t0 < 0), select(max(t0, t1), t0, t0 < 0));
}
