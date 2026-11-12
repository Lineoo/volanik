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
    let dir_sun = vec3f(cos(rad_sun), 0, sin(rad_sun));

    let clouds = fs_clouds(p_near, dir, dir_sun);
    let ground = fs_ground(p_near, dir, dir_sun);
    let sky = fs_sky(p_near, dir, dir_sun);
    return clouds + (ground + sky * (1 - ground.a)) * (1 - clouds.a);
}

fn fs_clouds(p: vec3f, dir: vec3f, dir_sun: vec3f) -> vec4f {
    const MARCHING_STEPS = 8;
    const LIGHT_STEPS = 4;
        
    let sun_phase = henyey_greenstein(-dot(dir, dir_sun), -0.6);

    var color = vec3f();
    var dens = 0.0;
    let s_low = spherical_intersect_bi(p, dir, 4000);
    let s_high = spherical_intersect_bi(p, dir, 10000);
    var s = max(select(s_high.x, s_low.y, s_low.x < 0.0), 0.0);
    if s > 300e3 { return vec4f(); }
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let p_cloud = p + dir * s;
        let ds = 1e3 * (hash14(vec4f(dir * 23.24, f32(i) * 21.5)) * 2);
        // * hash14(vec4f(dir * 23.24, f32(i) * 21.5)) * 2;
        let density = cloud_density(p_cloud);

        var dens_sun = 0.0;
        var s_sun = 0.0;
        for (var j = 0; j < LIGHT_STEPS; j++) {
            let p_sun = p_cloud + dir_sun * s_sun;
            let ds_sun = 1e2 * hash14(vec4f(dir * 63.22, f32(j) * 17.3)) * 2;
            // * hash14(vec4f(dir * 63.22, f32(j) * 17.3)) * 2;
            dens_sun += cloud_density(p_sun) * ds_sun;
            s_sun += ds_sun;
        }

        let irradiation_in = sun_phase * IRRADIATION_RATE * exp(-(CLOUD_ABSORB + CLOUD_SCATTER) * dens_sun);
        let transmission = exp(-(CLOUD_ABSORB + CLOUD_SCATTER) * dens);

        color += irradiation_in * transmission * CLOUD_SCATTER * density * ds;
        dens += density * ds;
        s += ds;
    }

    let transmission = exp(-(CLOUD_ABSORB + CLOUD_SCATTER) * dens);
    let a = 1 - (transmission.x + transmission.y + transmission.z) / 3.;
    return vec4f(color, a);
}

// Rayleigh scatter
fn fs_sky(p_near: vec3f, dir_view: vec3f, dir_sun: vec3f) -> vec4f {
    const MARCHING_STEPS = 8;
    const LIGHT_STEPS = 8;

    var total_rayleigh = vec3f();
    let dir_cos = dot(dir_view, dir_sun);
    let phase_r = 3.0 / (16.0 * PI) * (1 + dir_cos * dir_cos);

    var od_view = 0.0;
    let ds_view = spherical_intersect(p_near, dir_view, ATMOS_RADIUS) / MARCHING_STEPS;
    for (var i = 0; i < MARCHING_STEPS; i++) {
        let p_view = p_near + dir_view * ds_view * f32(i);
        od_view += atmos_density(p_view) * ds_view;

        var od_sun = 0.0;
        let ds_sun = spherical_intersect(p_view, dir_sun, ATMOS_RADIUS) / LIGHT_STEPS;
        for (var j = 0; j < LIGHT_STEPS; j++) {
            let p_sun = p_view + dir_sun * ds_sun * f32(j);
            od_sun += atmos_density(p_sun) * ds_sun;
        }

        let scatter = BETA_RAY * phase_r * atmos_density(p_view);
        let transmission = exp(-BETA_RAY * (od_view + od_sun));
        total_rayleigh += transmission * IRRADIATION_RATE * scatter * ds_view;
    }

    return clamp(vec4f(total_rayleigh, 1.0), vec4f(0), vec4f(1));
}

fn fs_ground(p: vec3f, dir: vec3f, dir_sun: vec3f) -> vec4f {
    const GROUND_HEIGHT = -10.0;
    const LIGHT_STEPS = 2;

    let s = spherical_intersect(p, dir, GROUND_HEIGHT);
    if s <= 0 { return vec4f(); }

    let p_ground = p + dir * s;
    let ambient_grassland = mix(vec3f(0.4, 0.6, 0.2), vec3f(0.3, 0.3, 0.1), fbm4_vnoise3(p_ground / 100)) / 20;
    let ambient_water = vec3f(0.2, 0.2, 0.6) / 20;
    let water_feature = step(0, fbm4_pnoise4(vec4f(p_ground / 1e6, 0.0)));
    let ambient = mix(ambient_grassland, ambient_water, water_feature);

    var od_sun = 0.0;
    let ds_sun = spherical_intersect(p_ground, dir_sun, ATMOS_RADIUS) / LIGHT_STEPS;
    for (var j = 0; j < LIGHT_STEPS; j++) {
        let p_sun = p_ground + dir_sun * ds_sun * f32(j);
        od_sun += atmos_density(p_sun) * ds_sun;
    }

    let transmission = exp(-BETA_RAY * od_sun) * IRRADIATION_RATE;

    return vec4f(transmission * ambient, 1.0);
}

// Planet //

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

fn spherical_height(p: vec3f) -> f32 {
    // height = length(p - C) - PLANET_RADIUS
    let q = vec3f(p.x, p.y, p.z + PLANET_RADIUS);
    let r = length(q);

    // h = r - R = (r^2 - R^2) / (r + R)
    let numerator = fma(p.z, p.z + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    return numerator / (r + PLANET_RADIUS);
}

// Atmosphere //

const ATMOS_RADIUS = 100e3;
const BETA_RAY = vec3f(5.5e-6, 13.0e-6, 22.4e-6);
const IRRADIATION_RATE = vec3f(20);

fn atmos_density(p: vec3f) -> f32 {
    const H_RAY = 8000;
    return exp(-spherical_height(p) / H_RAY);
}

// Clouds //

const CLOUD_SCATTER = vec3f(15e-3);
const CLOUD_ABSORB = vec3f();

// c: cos(t), g: IOR
fn henyey_greenstein(c: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1 - g2) / pow(1 + g2 - 2 * g * c, 3 / 2) / 4 / PI;
}

fn cloud_density(p: vec3f) -> f32 {
    let height = spherical_height(p);
    let mask = smoothstep(5000, 6000, height) * smoothstep(10000, 9000, height);
    return step(0.5, fbm4_vnoise3(p * 0.7e-3)) * mask;
}