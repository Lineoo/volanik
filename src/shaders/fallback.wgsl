struct VertexOutput {
    @builtin(position) pos: vec4f,
    @location(0) ndc: vec2f,
}

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
    return vec4f(0.5, 0.5, 0.5, 1);
}