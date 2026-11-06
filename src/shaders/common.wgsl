struct Scene {
    view: mat4x4f,
    time: f32,
}

const PI                : f32 = 3.14159265358979323846264338327950288;
const TAU               : f32 = 6.28318530717958647692528676655900577;
const FRAC_1_SQRT_PI    : f32 = 0.564189583547756286948079451560772586;
const FRAC_1_SQRT_TAU   : f32 = 0.398942280401432677939946059934381868;
const FRAC_1_PI         : f32 = 0.318309886183790671537767526745028724;
const FRAC_1_TAU        : f32 = 0.159154943091895335768883763372514362;
const FRAC_1_SQRT_2     : f32 = 0.707106781186547524400844362104849039;

const WHITE: vec4f = vec4f(1);
const BLACK: vec4f = vec4f(vec3f(0), 1);

// Hash without Sine (Ported)
// Copyright (c) 2014 David Hoskins. MIT License. https://www.shadertoy.com/view/4djSRW

fn hash11(p_in: f32) -> f32 {
    var p = p_in;
    p = fract(p * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash13(p_in: vec3f) -> f32 {
    var p3 = fract(p_in * 0.1031);
    p3 += dot(p3, p3.zyx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash14(p_in: vec4f) -> f32 {
    var p4 = fract(p_in * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    p4 += dot(p4, p4.wzxy + 33.33);
    return fract((p4.x + p4.y) * (p4.z + p4.w));
}

fn hash21(p: f32) -> vec2f {
    var p3 = fract(vec3f(p) * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn hash22(p: vec2f) -> vec2f {
    var p3 = fract(vec3f(p.xyx) * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn hash23(p_in: vec3f) -> vec2f {
    var p3 = fract(p_in * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn hash31(p: f32) -> vec3f {
    var p3 = fract(vec3f(p) * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

fn hash32(p: vec2f) -> vec3f {
    var p3 = fract(vec3f(p.xyx) * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

fn hash33(p_in: vec3f) -> vec3f {
    var p3 = fract(p_in * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yxx) * p3.zyx);
}

fn hash41(p: f32) -> vec4f {
    var p4 = fract(vec4f(p) * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    p4 += dot(p4, p4.wzxy + 33.33);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

fn hash42(p: vec2f) -> vec4f {
    var p4 = fract(vec4f(p.xyxy) * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    p4 += dot(p4, p4.wzxy + 33.33);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

fn hash43(p: vec3f) -> vec4f {
    var p4 = fract(vec4f(p.xyzx) * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    p4 += dot(p4, p4.wzxy + 33.33);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

fn hash44(p_in: vec4f) -> vec4f {
    var p4 = fract(p_in * vec4f(0.1031, 0.1030, 0.0973, 0.1099));
    p4 += dot(p4, p4.wzxy + 33.33);
    return fract((p4.xxyz + p4.yzzw) * p4.zywx);
}

// 2D Value Noise (Cubic Interpolate)
fn vnoise2(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n00 = hash12(i + vec2f(0, 0));
    let n10 = hash12(i + vec2f(1, 0));
    let n01 = hash12(i + vec2f(0, 1));
    let n11 = hash12(i + vec2f(1, 1));
    let nx0 = mix(n00, n10, u.x);
    let nx1 = mix(n01, n11, u.x);
    return mix(nx0, nx1, u.y);
}

// 3D Value Noise (Cubic Interpolate)
fn vnoise3(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash13(i + vec3f(0, 0, 0));
    let n100 = hash13(i + vec3f(1, 0, 0));
    let n010 = hash13(i + vec3f(0, 1, 0));
    let n110 = hash13(i + vec3f(1, 1, 0));
    let n001 = hash13(i + vec3f(0, 0, 1));
    let n101 = hash13(i + vec3f(1, 0, 1));
    let n011 = hash13(i + vec3f(0, 1, 1));
    let n111 = hash13(i + vec3f(1, 1, 1));
    let nx00 = mix(n000, n100, u.x);
    let nx10 = mix(n010, n110, u.x);
    let nx01 = mix(n001, n101, u.x);
    let nx11 = mix(n011, n111, u.x);
    let nxy0 = mix(nx00, nx10, u.y);
    let nxy1 = mix(nx01, nx11, u.y);
    return mix(nxy0, nxy1, u.z);
}

// 4D Value Noise (Cubic Interpolate)
fn vnoise4(p: vec4f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n0000 = hash14(i + vec4f(0, 0, 0, 0));
    let n1000 = hash14(i + vec4f(1, 0, 0, 0));
    let n0100 = hash14(i + vec4f(0, 1, 0, 0));
    let n1100 = hash14(i + vec4f(1, 1, 0, 0));
    let n0010 = hash14(i + vec4f(0, 0, 1, 0));
    let n1010 = hash14(i + vec4f(1, 0, 1, 0));
    let n0110 = hash14(i + vec4f(0, 1, 1, 0));
    let n1110 = hash14(i + vec4f(1, 1, 1, 0));
    let n0001 = hash14(i + vec4f(0, 0, 0, 1));
    let n1001 = hash14(i + vec4f(1, 0, 0, 1));
    let n0101 = hash14(i + vec4f(0, 1, 0, 1));
    let n1101 = hash14(i + vec4f(1, 1, 0, 1));
    let n0011 = hash14(i + vec4f(0, 0, 1, 1));
    let n1011 = hash14(i + vec4f(1, 0, 1, 1));
    let n0111 = hash14(i + vec4f(0, 1, 1, 1));
    let n1111 = hash14(i + vec4f(1, 1, 1, 1));
    let nx000 = mix(n0000, n1000, u.x);
    let nx100 = mix(n0100, n1100, u.x);
    let nx010 = mix(n0010, n1010, u.x);
    let nx110 = mix(n0110, n1110, u.x);
    let nx001 = mix(n0001, n1001, u.x);
    let nx101 = mix(n0101, n1101, u.x);
    let nx011 = mix(n0011, n1011, u.x);
    let nx111 = mix(n0111, n1111, u.x);
    let nxy00 = mix(nx000, nx100, u.y);
    let nxy10 = mix(nx010, nx110, u.y);
    let nxy01 = mix(nx001, nx101, u.y);
    let nxy11 = mix(nx011, nx111, u.y);
    let nxyz0 = mix(nxy00, nxy10, u.z);
    let nxyz1 = mix(nxy01, nxy11, u.z);
    return mix(nxyz0, nxyz1, u.w);
}

// 4-step Fractal Brownian Motion with 2D Value Noise
fn fbm4_vnoise2(p: vec2f) -> f32 {
    var amp = 0.5;
    var frq = 1.0;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 4; i++) {
        sum = sum + vnoise2(p * frq) * amp;
        norm = norm + amp;
        amp = amp * 0.5;
        frq = frq * 2.0;
    }
    return sum / norm;
}

// 4-step Fractal Brownian Motion with 3D Value Noise
fn fbm4_vnoise3(p: vec3f) -> f32 {
    var amp = 0.5;
    var frq = 1.0;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 4; i++) {
        sum = sum + vnoise3(p * frq) * amp;
        norm = norm + amp;
        amp = amp * 0.5;
        frq = frq * 2.0;
    }
    return sum / norm;
}

// 4-step Fractal Brownian Motion with 4D Value Noise
fn fbm4_vnoise4(p: vec4f) -> f32 {
    var amp = 0.5;
    var frq = 1.0;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 4; i++) {
        sum = sum + vnoise4(p * frq) * amp;
        norm = norm + amp;
        amp = amp * 0.5;
        frq = frq * 2.0;
    }
    return sum / norm;
}