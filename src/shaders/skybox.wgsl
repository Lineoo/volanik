struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) ndc: vec2f,
}

@group(0) @binding(0)
var<uniform> scene: Scene;

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
    let p_near = scene.view * vec4f(in.ndc, 0.0, 1.0);
    let p_far  = scene.view * vec4f(in.ndc, 1.0, 1.0);
    let a = p_near.xyz / p_near.w;
    let b = p_far.xyz  / p_far.w;
    let dir = normalize(b - a);

    let ds = ((11 - a.z) / dir.z);
    let p = a + dir * ds;

    if ds > 0 {   
        return vec4f(vec3f(fbm4(p.xy + vec2f(scene.time, 0))), 1.0);
    } else {   
        return vec4f(vec3f(0.5), 1.0);
    }
}
