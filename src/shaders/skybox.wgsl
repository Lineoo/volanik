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
    const DAY_CYCLE = 24 * 60 * 60;

    let rot_world = mat3x3f(
        scene.world[0].xyz,
        scene.world[1].xyz,
        scene.world[2].xyz
    );

    let pv_far = scene.view * vec4f(in.ndc, 1.0, 1.0);
    let pv_near = scene.view * vec4f(in.ndc, 0.0, 1.0);
    let pw_far = pv_far / pv_far.w;
    let pw_near = pv_near / pv_near.w;
    let p_far = (scene.world * pw_far).xyz;
    let p_near = (scene.world * pw_near).xyz;
    let dir = normalize((rot_world * (pw_far - pw_near).xyz));

    let rad_sun = scene.time / DAY_CYCLE * TAU;

    let ground = clamp(fs_ground(p_near, dir), vec4f(0), vec4f(1));
    let clouds = clamp(fs_clouds(p_near, dir), vec4f(0), vec4f(1));
    let sky = clamp(fs_sky(p_near, dir, vec3f(cos(rad_sun), 0, sin(rad_sun))), vec4f(0), vec4f(1));
    return clouds + (ground + sky * (1 - ground.a)) * (1 - clouds.a);
}

// Rayleigh scatter
fn fs_sky(p_near: vec3f, dir_view: vec3f, dir_sun: vec3f) -> vec4f {
    const ATMOS_RADIUS = 100e3;
    const IRRADIATION_RATE = vec3f(10);
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
    const MARCHING_STEPS = 4;
    const SUB_STEPS = 4;

    let s_low = spherical_intersect_bi(p, dir, CLOUD_LOW_HEIGHT);
    let s_high = spherical_intersect_bi(p, dir, CLOUD_HIGH_HEIGHT);
    let s_near = max(select(s_high.x, s_low.y, s_low.x < 0.0), 0.0);
    let s_far = min(select(s_high.y, s_low.x, s_low.x > 0.0 && s_low.x < s_high.y), 1e5);
    let p_near = p + dir * s_near;

    if s_high.y < 0 {
        return vec4f();
    }

    var sum = 0.0;
    let ds = min((s_far - s_near) / MARCHING_STEPS, 5e3);
    let noise_f = vec3f(2e-4, 2e-4, 6e-4);
    let noise_s = vec3f(0, scene.time * 1e-3, 0);
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let p_first = p_near + dir * ds * f32(i) + dir * ds * (hash14(vec4f(dir * 23.24, f32(i) * 21.5)) - 0.5);
        let noise = fbm4_vnoise3(p_first * noise_f + noise_s)
            * smoothstep(CLOUD_LOW_HEIGHT, CLOUD_LOW_HEIGHT + 2e2, p_first.z)
            * smoothstep(CLOUD_HIGH_HEIGHT, CLOUD_HIGH_HEIGHT - 5e2, p_first.z);
        if noise > 0.4 {
            let ds_sub = ds / SUB_STEPS;
            for (var j = 0; j < SUB_STEPS; j++) {
                let p_sub = p_first + dir * ds_sub * f32(j) + dir * ds_sub * (hash14(vec4f(dir * 23.24, f32(j) * 21.5)) - 0.5);
                let noise2 = fbm4_vnoise3(p_sub * noise_f + noise_s)
                    * smoothstep(CLOUD_LOW_HEIGHT, CLOUD_LOW_HEIGHT + 2e2, p_sub.z)
                    * smoothstep(CLOUD_HIGH_HEIGHT, CLOUD_HIGH_HEIGHT - 5e2, p_sub.z);
                let density = smoothstep(0.6, 0.62, noise2);
                sum += density * ds_sub;
            } 
        }
    }

    return vec4f(1 - exp(-5e-4 * sum));
}

fn fs_ground(p: vec3f, dir: vec3f) -> vec4f {
    const GROUND_HEIGHT = -10;

    let s = spherical_intersect(p, dir, GROUND_HEIGHT);
    let p_ground = p + dir * s;
    let color = mix(vec4(0.4, 0.6, 0.2, 1.0), vec4(0.3, 0.3, 0.1, 1.0), fbm4_vnoise3(p_ground / 100));
    return select(vec4f(), color, s > 0);
}

const PLANET_RADIUS = 6371e3;

// r^2 = a^2 + b^2 - 2abcos
// b^2 - (2acos) b + (a^2 - r^2) = 0
// delta = 4(acos)^2 - 4 (a^2 - r^2)
// t = (- 2acos +/- sqrt(delta)) / 2
// t = - acos +/- sqrt(delta / 4)
//
// precise loss: PLANET_RADIUS (big)
fn spherical_intersect(p: vec3f, d: vec3f, w: f32) -> f32 {
    let t = spherical_intersect_bi(p, d, w);
    return select(min(t.x, t.y), t.y, t.x < 0);
}

fn spherical_intersect_bi(p: vec3f, d: vec3f, w: f32) -> vec2f {
    let a_cos = fma(PLANET_RADIUS, d.z, dot(p, d));
    let c = fma(p.z - w, p.z + w + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    let frac_d4 = fma(a_cos, a_cos, -c);
    if (frac_d4 < 0) { return vec2f(); }
    let sq = sqrt(frac_d4);
    let t0 = -a_cos - sq;
    let t1 = -a_cos + sq;
    return vec2f(t0, t1);
}
