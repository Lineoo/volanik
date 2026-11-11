struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) ndc: vec2f,
}

@group(0) @binding(0) var<uniform> scene: Scene;

const MAX_CLOUD_DISTANCE: f32 = 400e3;
const GROUND_HEIGHT: f32 = -10.0;

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

    // Clouds are only allowed up to the first opaque thing behind them.
    let s_ground = ground_distance(p_near, dir);
    let t_max = select(MAX_CLOUD_DISTANCE, s_ground, s_ground > 0.0);
    let clouds = clamp(
        fs_clouds(p_near, dir, dir_sun, t_max, hash12(in.pos.xy)),
        vec4f(0),
        vec4f(1),
    );

    if (clouds.a >= 0.999) {
        return vec4f(clouds.rgb, 1.0);
    }

    let ground = clamp(fs_ground(p_near, dir, dir_sun, s_ground), vec4f(0), vec4f(1));
    let sky = clamp(fs_sky(p_near, dir, dir_sun), vec4f(0), vec4f(1));
    return clouds + (ground + sky * (1 - ground.a)) * (1 - clouds.a);
}

const PLANET_RADIUS = 6371e3;
const H_RAY = 8000;
const ATMOS_RADIUS = 100e3;
const BETA_RAY = vec3f(5.5e-6, 13.0e-6, 22.4e-6);
// const BETA_RAY = vec3f(5.5e-6, 24.0e-6, 22.4e-6);
const IRRADIATION_RATE = vec3f(20);

// cloud shape / lighting constants
const WEATHER_SCALE: f32 = 1.0 / 8000.0;
const CLOUD_SCALE: f32 = 1.0 / 900.0;
const DETAIL_SCALE: f32 = 1.0 / 400.0;
const BILLOW_SCALE: f32 = 1.0 / 1100.0;
const BILLOW2_SCALE: f32 = 1.0 / 550.0;
const CLOUD_EXTINCTION: f32 = 0.0035;
const CLOUD_SCATTER: f32 = 0.0022;

const CLOUD_WIND_DIR = vec3f(1.0, 0.35, 0.05);

fn cloud_wind_offset() -> vec3f {
    return CLOUD_WIND_DIR * (scene.cloud_wind * scene.time);
}

// Rotate + scale between octaves so value noise doesn't show its grid.
fn fbm2_rot(p_in: vec2f, octaves: u32) -> f32 {
    var p = p_in;
    var amp = 0.5;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0u; i < octaves; i++) {
        sum += vnoise2(p) * amp;
        norm += amp;
        p = vec2f(p.x * 1.6 - p.y * 1.2, p.x * 1.2 + p.y * 1.6) + vec2f(13.7, 7.1);
        amp *= 0.55;
    }
    return sum / norm;
}

fn fbm3_rot(p_in: vec3f, octaves: u32) -> f32 {
    var p = p_in;
    var amp = 0.5;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0u; i < octaves; i++) {
        sum += vnoise3(p) * amp;
        norm += amp;
        p = mat3x3f(
            vec3f(1.6, -1.2, 0.0),
            vec3f(1.2, 1.6, 0.0),
            vec3f(0.0, 0.0, 2.0)
        ) * p + vec3f(13.7, 7.1, 3.3);
        amp *= 0.5;
    }
    return sum / norm;
}

fn sky_ambient(dir_sun: vec3f, hn: f32) -> vec3f {
    let day = smoothstep(-0.08, 0.25, dir_sun.z);
    let zenith = vec3f(0.18, 0.32, 0.65);
    let horizon = vec3f(0.55, 0.62, 0.72);
    let sky = mix(horizon, zenith, smoothstep(0.0, 0.8, hn));
    let ground_bounce = vec3f(0.12, 0.13, 0.14);
    // A cloud base sees a lot of sky and some ground bounce.
    let base = mix(ground_bounce, sky, 0.45);

    // warm ambient near sunset
    let sunset = 1.0 - smoothstep(0.0, 0.30, abs(dir_sun.z));
    let tint = mix(vec3f(1.0), vec3f(1.0, 0.70, 0.45), sunset);

    return mix(base, sky, smoothstep(0.0, 0.45, hn))
        * (0.15 + 0.85 * day) * 1.1 * tint;
}

fn cloud_density(p: vec3f, octaves: u32, detail: bool) -> f32 {
    let base = scene.cloud_base;
    let top = max(scene.cloud_top, base + 500.0);
    let h = spherical_height(p);
    let hn = clamp((h - base) / (top - base), 0.0, 1.0);

    // flat base, rounded top
    let profile = smoothstep(0.0, 0.02, hn) * (1.0 - smoothstep(0.55, 1.0, hn));
    if (profile <= 0.0) {
        return 0.0;
    }

    let wind = cloud_wind_offset();

    // 2D weather map: decides where clouds are, and where the gaps are.
    let weather = fbm2_rot((p.xy + wind.xy) * WEATHER_SCALE, 3u);
    let coverage = clamp(scene.cloud_coverage, 0.0, 1.0);
    let cov = smoothstep(1.0 - coverage, 1.0 - coverage + 0.25, weather);
    // cumulus narrow towards the top
    let cov_h = cov * (1.0 - 0.4 * smoothstep(0.2, 1.0, hn));

    // 3D base shape: value fbm for the large mass, Worley billows for
    // the cellular cumulus structure.
    let fbm = fbm3_rot((p + wind) * CLOUD_SCALE, octaves);
    let billow = 1.0 - worley3_f1((p + wind) * BILLOW_SCALE);
    let billow2 = 1.0 - worley3_f1((p + wind * 1.2) * BILLOW2_SCALE);

    // Push the isosurface around at a finer scale; this is what makes the
    // silhouette read as cauliflower instead of a smooth blob.
    var shape = fbm * 0.55 + billow * 0.30 + billow2 * 0.15;
    var erode = 0.0;
    if (detail) {
        erode = vnoise3((p + wind * 1.5) * DETAIL_SCALE) - 0.5;
        shape += erode * 0.28;
    }

    var density = saturate((shape - (1.0 - cov_h)) * 6.0) * profile;

    // additionally erode the thin edges
    if (detail && density > 0.01) {
        density = saturate(density - erode * 0.55 * (1.0 - density));
    }

    return saturate(density * scene.cloud_density);
}

fn cloud_lighting(
    p: vec3f,
    dir: vec3f,
    dir_sun: vec3f,
    density: f32,
    sun_color: vec3f,
    light_steps: u32,
) -> vec3f {
    const LIGHT_STEP = 400.0;

    var od = 0.0;
    var q = p;
    for (var i = 0u; i < light_steps; i++) {
        q += dir_sun * LIGHT_STEP;
        od += cloud_density(q, 2u, true) * LIGHT_STEP;
    }

    let E = exp(-od * CLOUD_EXTINCTION);
    // Cheap multiple scattering approximation (Schneider et al.).
    let ms = E + 0.6 * E * E + 0.4 * E * E * E + 0.2 * E * E * E * E;

    let cos_theta = dot(dir, dir_sun);
    let phase = mix(
        henyey_greenstein(cos_theta, 0.65),
        henyey_greenstein(cos_theta, -0.2),
        0.25
    ) + 0.22; // isotropic multiple-scattering / diffuse fill

    let base = scene.cloud_base;
    let top = max(scene.cloud_top, base + 500.0);
    let hn = clamp((spherical_height(p) - base) / (top - base), 0.0, 1.0);
    let ambient = sky_ambient(dir_sun, hn) * (0.65 + 0.35 * E);

    // Darken thin edges; without this the cauliflower edges look like fog.
    let powder = 1.0 - exp(-density * 4.0);
    let direct = sun_color * phase * ms * powder;

    return direct + ambient;
}

fn sun_transmittance(p: vec3f, dir_sun: vec3f) -> vec3f {
    const STEPS = 4;

    let horizon = smoothstep(-0.04, 0.04, dir_sun.z);
    if (horizon <= 0.0) {
        return vec3f();
    }

    let dist = max(spherical_intersect(p, dir_sun, ATMOS_RADIUS), 0.0);
    let ds = dist / f32(STEPS);

    var od = 0.0;
    for (var i = 0; i < STEPS; i++) {
        let q = p + dir_sun * ds * (f32(i) + 0.5);
        od += atmos_density(q) * ds;
    }

    return exp(-BETA_RAY * od) * horizon;
}

fn atmos_transmittance(p: vec3f, dir: vec3f, dist: f32) -> vec3f {
    const STEPS = 4;

    if (dist <= 0.0) {
        return vec3f(1.0);
    }

    let ds = dist / f32(STEPS);
    var od = 0.0;
    for (var i = 0; i < STEPS; i++) {
        let q = p + dir * ds * (f32(i) + 0.5);
        od += atmos_density(q) * ds;
    }

    return exp(-BETA_RAY * od);
}

// Same quadratic as spherical_intersect_bi, but with an explicit hit flag.
// z < 0 means the ray missed the sphere.
fn sphere_intersect_hit(p: vec3f, d: vec3f, w: f32) -> vec3f {
    let a_cos = fma(PLANET_RADIUS, d.z, dot(p, d));
    let c = fma(p.z - w, p.z + w + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    let frac_d4 = fma(a_cos, a_cos, -c);
    if (frac_d4 < 0.0) {
        return vec3f(0.0, 0.0, -1.0);
    }
    let sq = sqrt(frac_d4);
    return vec3f(-a_cos - sq, -a_cos + sq, 1.0);
}

fn cloud_interval(p: vec3f, dir: vec3f, h0: f32, h1: f32, t_max: f32) -> vec2f {
    let lo = sphere_intersect_hit(p, dir, h0);
    let hi = sphere_intersect_hit(p, dir, h1);
    if (lo.z < 0.0 || hi.z < 0.0) {
        return vec2f(-1.0);
    }

    let h = spherical_height(p);
    var t0: f32;
    var t1: f32;
    if (h < h0) {
        t0 = lo.y;
        t1 = hi.y;
    } else if (h < h1) {
        t0 = 0.0;
        t1 = hi.y;
    } else {
        t0 = hi.x;
        t1 = lo.x;
    }

    t0 = max(t0, 0.0);
    t1 = min(t1, t_max);
    if (t1 <= t0) {
        return vec2f(-1.0);
    }
    return vec2f(t0, t1);
}

fn fs_clouds(
    p: vec3f,
    dir: vec3f,
    dir_sun: vec3f,
    t_max: f32,
    jitter: f32,
) -> vec4f {
    let base = scene.cloud_base;
    let top = max(scene.cloud_top, base + 500.0);
    let interval = cloud_interval(p, dir, base, top, t_max);
    if (interval.x < 0.0) {
        return vec4f();
    }

    let t0 = interval.x;
    let t1 = interval.y;

    let quality = clamp(scene.cloud_quality, 0.0, 1.0);
    let view_steps = u32(mix(24.0, 80.0, quality));
    let light_steps = u32(mix(4.0, 8.0, quality));
    let octaves = u32(mix(3.0, 5.0, quality));

    let ds = (t1 - t0) / f32(view_steps);
    let detail = ds < 250.0;

    // LOD: never sample frequencies the step size can't resolve.
    var shape_octaves = octaves;
    if (ds > 120.0) {
        shape_octaves = min(shape_octaves, 3u);
    }
    if (ds > 250.0) {
        shape_octaves = min(shape_octaves, 2u);
    }
    if (ds > 700.0) {
        shape_octaves = 1u;
    }

    let p_entry = p + dir * t0;
    let sun_color = IRRADIATION_RATE * sun_transmittance(p_entry, dir_sun);
    let view_t = atmos_transmittance(p, dir, t0);

    var t = t0 + jitter * ds;
    var transmittance = 1.0;
    var color = vec3f();

    for (var i = 0u; i < view_steps; i++) {
        if (t >= t1) {
            break;
        }

        let q = p + dir * t;
        let density = cloud_density(q, shape_octaves, detail);
        if (density > 0.002) {
            let light = cloud_lighting(q, dir, dir_sun, density, sun_color, light_steps);
            let step_t = exp(-CLOUD_EXTINCTION * density * ds);

            color += transmittance * light * (CLOUD_SCATTER * density * ds);
            transmittance *= step_t;

            if (transmittance < 0.01) {
                break;
            }
        }

        t += ds;
    }

    // Soft shoulder: keep shape in the sunlit tops instead of clipping to white.
    let tonemapped = vec3f(1.0) - exp(-color * 1.6);

    // Aerial perspective: fade both colour and alpha towards the sky so
    // distant clouds don't turn into black silhouettes.
    let distance_fade = 1.0 - smoothstep(40e3, 220e3, t0);
    // Very coarse steps at the horizon produce speckle, not clouds.
    let step_fade = 1.0 - smoothstep(1500.0, 4000.0, ds);
    let fade = distance_fade * step_fade;
    let alpha = (1.0 - transmittance)
        * dot(view_t, vec3f(0.299, 0.587, 0.114))
        * fade;
    return vec4f(tonemapped * view_t * fade, alpha);
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

    return vec4f(total_rayleigh, 1.0);
}

fn ground_distance(p: vec3f, dir: vec3f) -> f32 {
    let s = spherical_intersect(p, dir, GROUND_HEIGHT);
    return select(-1.0, s, s > 0.0);
}

fn fs_ground(p: vec3f, dir: vec3f, dir_sun: vec3f, s: f32) -> vec4f {
    const LIGHT_STEPS = 2;

    if (s <= 0.0) {
        return vec4f();
    }

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

// r^2 = a^2 + b^2 - 2abcos
// b^2 - (2acos) b + (a^2 - r^2) = 0
// delta = 4(acos)^2 - 4 (a^2 - r^2)
// t = (- 2acos +/- sqrt(delta)) / 2
// t = - acos +/- sqrt(delta / 4)
//
// precise loss: PLANET_RADIUS (big)
fn spherical_intersect(p: vec3f, d: vec3f, w: f32) -> f32 {
    let t = sphere_intersect_hit(p, d, w);
    if (t.z < 0.0) {
        return -1.0;
    }
    return select(min(t.x, t.y), t.y, t.x < 0);
}

fn spherical_height(p: vec3f) -> f32 {
    // height = length(p - C) - PLANET_RADIUS
    let q = vec3f(p.x, p.y, p.z + PLANET_RADIUS);
    let r = length(q);

    // h = r - R = (r^2 - R^2) / (r + R)
    let numerator = fma(p.z, p.z + 2.0 * PLANET_RADIUS, dot(p.xy, p.xy));
    return numerator / (r + PLANET_RADIUS);
}

// c: cos(t), g: IOR
fn henyey_greenstein(c: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * PI * pow(1.0 + g2 - 2.0 * g * c, 1.5));
}

fn atmos_density(p: vec3f) -> f32 {
    return exp(-spherical_height(p) / H_RAY);
}
