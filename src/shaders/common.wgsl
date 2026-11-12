struct Scene {
    view: mat4x4f,
    world: mat4x4f,
    time: f32,
}

const E                 : f32 = 2.71828182845904523536028747135266250;
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

// 4D Checker Texture
fn checker4(p: vec4f) -> f32 {
    let i = vec4i(floor(p));
    return f32((i.x + i.y + i.z + i.w) & 1);
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

// 4D Value Noise (Cubic Gradient)
fn vnoise4_grad(p: vec4f) -> vec4f {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let du = 6 * f * (1 - f);
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

    let gx = du.x * (
        (n1000 - n0000) * (1.0-u.y) * (1.0-u.z) * (1.0-u.w)
      + (n1100 - n0100) * u.y       * (1.0-u.z) * (1.0-u.w)
      + (n1010 - n0010) * (1.0-u.y) * u.z       * (1.0-u.w)
      + (n1110 - n0110) * u.y       * u.z       * (1.0-u.w)
      + (n1001 - n0001) * (1.0-u.y) * (1.0-u.z) * u.w
      + (n1101 - n0101) * u.y       * (1.0-u.z) * u.w
      + (n1011 - n0011) * (1.0-u.y) * u.z       * u.w
      + (n1111 - n0111) * u.y       * u.z       * u.w
    );

    let gy = du.y * (
        (n0100 - n0000) * (1.0-u.x) * (1.0-u.z) * (1.0-u.w)
      + (n1100 - n1000) * u.x       * (1.0-u.z) * (1.0-u.w)
      + (n0110 - n0010) * (1.0-u.x) * u.z       * (1.0-u.w)
      + (n1110 - n1010) * u.x       * u.z       * (1.0-u.w)
      + (n0101 - n0001) * (1.0-u.x) * (1.0-u.z) * u.w
      + (n1101 - n1001) * u.x       * (1.0-u.z) * u.w
      + (n0111 - n0011) * (1.0-u.x) * u.z       * u.w
      + (n1111 - n1011) * u.x       * u.z       * u.w
    );

    let gz = du.z * (
        (n0010 - n0000) * (1.0-u.x) * (1.0-u.y) * (1.0-u.w)
      + (n1010 - n1000) * u.x       * (1.0-u.y) * (1.0-u.w)
      + (n0110 - n0100) * (1.0-u.x) * u.y       * (1.0-u.w)
      + (n1110 - n1100) * u.x       * u.y       * (1.0-u.w)
      + (n0011 - n0001) * (1.0-u.x) * (1.0-u.y) * u.w
      + (n1011 - n1001) * u.x       * (1.0-u.y) * u.w
      + (n0111 - n0101) * (1.0-u.x) * u.y       * u.w
      + (n1111 - n1101) * u.x       * u.y       * u.w
    );

    let gw = du.w * (
        (n0001 - n0000) * (1.0-u.x) * (1.0-u.y) * (1.0-u.z)
      + (n1001 - n1000) * u.x       * (1.0-u.y) * (1.0-u.z)
      + (n0101 - n0100) * (1.0-u.x) * u.y       * (1.0-u.z)
      + (n1101 - n1100) * u.x       * u.y       * (1.0-u.z)
      + (n0011 - n0010) * (1.0-u.x) * (1.0-u.y) * u.z
      + (n1011 - n1010) * u.x       * (1.0-u.y) * u.z
      + (n0111 - n0110) * (1.0-u.x) * u.y       * u.z
      + (n1111 - n1110) * u.x       * u.y       * u.z
    );

    return vec4f(gx, gy, gz, gw);
}

fn grad4d(h: u32, o: vec4f) -> f32 {
    let k  = h & 31u;
    let u  = select(o.y, o.x, k < 24u);
    let v  = select(o.z, o.y, k < 16u);
    let s  = select(o.w, o.z, k <  8u);
    let su = select(u, -u, (k & 1u) != 0u);
    let sv = select(v, -v, (k & 2u) != 0u);
    let ss = select(s, -s, (k & 4u) != 0u);
    return su + sv + ss;
}

fn grad_hash14(cell: vec4i) -> u32 {
    return u32(hash14(vec4f(cell)) * 256.0) & 15u;
}

// 4D Perlin Noise (Quintic Interlope)
fn pnoise4(p: vec4f) -> f32 {
    let i = floor(p);
    let f  = fract(p);
    let u  = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let c = vec4i(i);
    let n0000 = grad4d(grad_hash14(c + vec4i(0, 0, 0, 0)), f - vec4f(0, 0, 0, 0));
    let n1000 = grad4d(grad_hash14(c + vec4i(1, 0, 0, 0)), f - vec4f(1, 0, 0, 0));
    let n0100 = grad4d(grad_hash14(c + vec4i(0, 1, 0, 0)), f - vec4f(0, 1, 0, 0));
    let n1100 = grad4d(grad_hash14(c + vec4i(1, 1, 0, 0)), f - vec4f(1, 1, 0, 0));
    let n0010 = grad4d(grad_hash14(c + vec4i(0, 0, 1, 0)), f - vec4f(0, 0, 1, 0));
    let n1010 = grad4d(grad_hash14(c + vec4i(1, 0, 1, 0)), f - vec4f(1, 0, 1, 0));
    let n0110 = grad4d(grad_hash14(c + vec4i(0, 1, 1, 0)), f - vec4f(0, 1, 1, 0));
    let n1110 = grad4d(grad_hash14(c + vec4i(1, 1, 1, 0)), f - vec4f(1, 1, 1, 0));
    let n0001 = grad4d(grad_hash14(c + vec4i(0, 0, 0, 1)), f - vec4f(0, 0, 0, 1));
    let n1001 = grad4d(grad_hash14(c + vec4i(1, 0, 0, 1)), f - vec4f(1, 0, 0, 1));
    let n0101 = grad4d(grad_hash14(c + vec4i(0, 1, 0, 1)), f - vec4f(0, 1, 0, 1));
    let n1101 = grad4d(grad_hash14(c + vec4i(1, 1, 0, 1)), f - vec4f(1, 1, 0, 1));
    let n0011 = grad4d(grad_hash14(c + vec4i(0, 0, 1, 1)), f - vec4f(0, 0, 1, 1));
    let n1011 = grad4d(grad_hash14(c + vec4i(1, 0, 1, 1)), f - vec4f(1, 0, 1, 1));
    let n0111 = grad4d(grad_hash14(c + vec4i(0, 1, 1, 1)), f - vec4f(0, 1, 1, 1));
    let n1111 = grad4d(grad_hash14(c + vec4i(1, 1, 1, 1)), f - vec4f(1, 1, 1, 1));

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

// 4D Perlin Noise (Quintic Gradient)
fn pnoise4_grad(p: vec4f) -> vec4f {
    let i = floor(p);
    let f  = fract(p);
    let u  = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let du = 30 * f * f * (f - 1) * (f - 1);
    let c = vec4i(i);
    let n0000 = grad4d(grad_hash14(c + vec4i(0, 0, 0, 0)), f - vec4f(0, 0, 0, 0));
    let n1000 = grad4d(grad_hash14(c + vec4i(1, 0, 0, 0)), f - vec4f(1, 0, 0, 0));
    let n0100 = grad4d(grad_hash14(c + vec4i(0, 1, 0, 0)), f - vec4f(0, 1, 0, 0));
    let n1100 = grad4d(grad_hash14(c + vec4i(1, 1, 0, 0)), f - vec4f(1, 1, 0, 0));
    let n0010 = grad4d(grad_hash14(c + vec4i(0, 0, 1, 0)), f - vec4f(0, 0, 1, 0));
    let n1010 = grad4d(grad_hash14(c + vec4i(1, 0, 1, 0)), f - vec4f(1, 0, 1, 0));
    let n0110 = grad4d(grad_hash14(c + vec4i(0, 1, 1, 0)), f - vec4f(0, 1, 1, 0));
    let n1110 = grad4d(grad_hash14(c + vec4i(1, 1, 1, 0)), f - vec4f(1, 1, 1, 0));
    let n0001 = grad4d(grad_hash14(c + vec4i(0, 0, 0, 1)), f - vec4f(0, 0, 0, 1));
    let n1001 = grad4d(grad_hash14(c + vec4i(1, 0, 0, 1)), f - vec4f(1, 0, 0, 1));
    let n0101 = grad4d(grad_hash14(c + vec4i(0, 1, 0, 1)), f - vec4f(0, 1, 0, 1));
    let n1101 = grad4d(grad_hash14(c + vec4i(1, 1, 0, 1)), f - vec4f(1, 1, 0, 1));
    let n0011 = grad4d(grad_hash14(c + vec4i(0, 0, 1, 1)), f - vec4f(0, 0, 1, 1));
    let n1011 = grad4d(grad_hash14(c + vec4i(1, 0, 1, 1)), f - vec4f(1, 0, 1, 1));
    let n0111 = grad4d(grad_hash14(c + vec4i(0, 1, 1, 1)), f - vec4f(0, 1, 1, 1));
    let n1111 = grad4d(grad_hash14(c + vec4i(1, 1, 1, 1)), f - vec4f(1, 1, 1, 1));

    let gx = du.x * (
        (n1000 - n0000) * (1.0-u.y) * (1.0-u.z) * (1.0-u.w)
      + (n1100 - n0100) * u.y       * (1.0-u.z) * (1.0-u.w)
      + (n1010 - n0010) * (1.0-u.y) * u.z       * (1.0-u.w)
      + (n1110 - n0110) * u.y       * u.z       * (1.0-u.w)
      + (n1001 - n0001) * (1.0-u.y) * (1.0-u.z) * u.w
      + (n1101 - n0101) * u.y       * (1.0-u.z) * u.w
      + (n1011 - n0011) * (1.0-u.y) * u.z       * u.w
      + (n1111 - n0111) * u.y       * u.z       * u.w
    );

    let gy = du.y * (
        (n0100 - n0000) * (1.0-u.x) * (1.0-u.z) * (1.0-u.w)
      + (n1100 - n1000) * u.x       * (1.0-u.z) * (1.0-u.w)
      + (n0110 - n0010) * (1.0-u.x) * u.z       * (1.0-u.w)
      + (n1110 - n1010) * u.x       * u.z       * (1.0-u.w)
      + (n0101 - n0001) * (1.0-u.x) * (1.0-u.z) * u.w
      + (n1101 - n1001) * u.x       * (1.0-u.z) * u.w
      + (n0111 - n0011) * (1.0-u.x) * u.z       * u.w
      + (n1111 - n1011) * u.x       * u.z       * u.w
    );

    let gz = du.z * (
        (n0010 - n0000) * (1.0-u.x) * (1.0-u.y) * (1.0-u.w)
      + (n1010 - n1000) * u.x       * (1.0-u.y) * (1.0-u.w)
      + (n0110 - n0100) * (1.0-u.x) * u.y       * (1.0-u.w)
      + (n1110 - n1100) * u.x       * u.y       * (1.0-u.w)
      + (n0011 - n0001) * (1.0-u.x) * (1.0-u.y) * u.w
      + (n1011 - n1001) * u.x       * (1.0-u.y) * u.w
      + (n0111 - n0101) * (1.0-u.x) * u.y       * u.w
      + (n1111 - n1101) * u.x       * u.y       * u.w
    );

    let gw = du.w * (
        (n0001 - n0000) * (1.0-u.x) * (1.0-u.y) * (1.0-u.z)
      + (n1001 - n1000) * u.x       * (1.0-u.y) * (1.0-u.z)
      + (n0101 - n0100) * (1.0-u.x) * u.y       * (1.0-u.z)
      + (n1101 - n1100) * u.x       * u.y       * (1.0-u.z)
      + (n0011 - n0010) * (1.0-u.x) * (1.0-u.y) * u.z
      + (n1011 - n1010) * u.x       * (1.0-u.y) * u.z
      + (n0111 - n0110) * (1.0-u.x) * u.y       * u.z
      + (n1111 - n1110) * u.x       * u.y       * u.z
    );

    return vec4f(gx, gy, gz, gw);
}

// 3D Worley noise First Point Distance
fn worley3_f1(p: vec3f) -> f32 {
    let cell  = floor(p);
    let local = fract(p);

    var minDist = 1e9;

    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let g = vec3f(f32(x), f32(y), f32(z));
                let o = hash33(cell + g);
                let r = g + o - local;
                minDist = min(minDist, dot(r, r));
            }
        }
    }
    return sqrt(minDist);
}

// 4-step Fractal Brownian Motion with 4D Perlin Noise
fn fbm4_pnoise4(p: vec4f) -> f32 {
    var amp = 0.5;
    var frq = 1.0;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 4; i++) {
        sum  = sum  + pnoise4(p * frq) * amp;
        norm = norm + amp;
        amp  = amp  * 0.5;
        frq  = frq  * 2.0;
    }
    return sum / norm;
}

// 4-step Fractal Brownian Motion with 2D Value Noise
fn fbm4_vnoise2(p0: vec2f) -> f32 {
    var p = p0;
    var amp = 0.5;
    var frq = 1.0;
    var sum = 0.0;
    var norm = 0.0;
    for (var i = 0; i < 4; i++) {
        sum  = sum + vnoise2(p * frq) * amp;
        norm = norm + amp;
        amp  = amp * 0.5;
        frq  = frq * 2.0;
        p = vec2f(p.x * 1.6 - p.y * 1.2, p.x * 1.2 + p.y * 1.6) + vec2f(13.7, 7.1);
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
        sum  = sum + vnoise3(p * frq) * amp;
        norm = norm + amp;
        amp  = amp * 0.5;
        frq  = frq * 2.0;
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
        sum  = sum + vnoise4(p * frq) * amp;
        norm = norm + amp;
        amp  = amp * 0.5;
        frq  = frq * 2.0;
    }
    return sum / norm;
}

fn gaussian1(n: f32) -> f32 {
    return FRAC_1_SQRT_TAU * exp(-(n * n) / 2.0);
}

fn gaussian2(n: vec2f) -> f32 {
    return FRAC_1_TAU * exp(-(n.x * n.x + n.y * n.y) / 2.0);
}
