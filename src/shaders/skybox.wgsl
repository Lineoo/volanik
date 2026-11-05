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

const CLOUD_HEIGHT = 100;

// 3600x fast forward
const DAY_CYCLE = 24 * 60 * 60 / 3600; 

const PLANET_RADIUS = 6371e3;
const ATMOS_RADIUS = 6471e3;
const BETA_RAY = vec3f(5.5e-6, 13.0e-6, 22.4e-6);
const H_RAY = 8000;
const MARCHING_STEPS = 32;
const LIGHT_STEPS = 8;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let pw_near = scene.view * vec4f(in.ndc, 0.0, 1.0);
    let pw_far = scene.view * vec4f(in.ndc, 1.0, 1.0);
    let p_near = pw_near.xyz / pw_near.w;
    let p_far = pw_far.xyz / pw_far.w;
    let dir = normalize(p_far - p_near);

    let ds = (CLOUD_HEIGHT - p_near.z) / dir.z;
    let p_cloud = p_near + dir * ds;

    let clouds = select(vec4f(fbm4(p_cloud.xy + vec2f(scene.time, 0))),
        vec4f(), ds <= 0);
    let rad_sun = scene.time / DAY_CYCLE * TAU;
    let sky = skybox(p_near, dir, vec3f(cos(rad_sun), 0, sin(rad_sun)));
    return clouds + sky * (1 - clouds.a);
}

// Raymarching
fn skybox(p_near: vec3f, dir_view: vec3f, dir_sun: vec3f) -> vec4f {
    var total_rayleigh = vec3f();
    let dir_cos = dot(dir_view, dir_sun);
    let phase_r = 3.0 / (16.0 * PI) * (1 + dir_cos * dir_cos);

    var od_view = 0.0;
    let ds_view = atmos_intersect(p_near, dir_view) / MARCHING_STEPS;
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let p_view = p_near + dir_view * ds_view * f32(i);
        od_view += exp(-p_view.z / H_RAY) * ds_view;

        var od_sun = 0.0;
        let ds_sun = atmos_intersect(p_view, dir_sun) / LIGHT_STEPS;
        for (var j = 0; j < LIGHT_STEPS; j++) {
            let p_sun = p_view + dir_sun * ds_sun * f32(j);
            od_sun += exp(-p_sun.z / H_RAY) * ds_sun;
        }

        let scatter = BETA_RAY * phase_r * exp(-p_view.z / H_RAY);
        let transmission = exp(-BETA_RAY * od_view) * exp(-BETA_RAY * od_sun);
        total_rayleigh += transmission * scatter * ds_view;
    }

    return vec4f(total_rayleigh, 1.0);
}

fn atmos_intersect(p: vec3f, d: vec3f) -> f32 {
    let o = p + vec3f(0, 0, PLANET_RADIUS);
    let b = dot(o, d);
    let c = dot(o, o) - ATMOS_RADIUS * ATMOS_RADIUS;
    let disc = b * b - c;
    if (disc < 0.0) { return -1.0; }
    let t = -b + sqrt(disc);
    return select(-1.0, t, t > 0.0);
}
