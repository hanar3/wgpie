// Vertex shader

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fragCoords: vec2<f32>,
};


fn sd_round_box(point: vec2<f32>, size: vec2<f32>, r: vec4<f32>)-> f32 {
    var radius: vec4<f32> = vec4<f32>(r);
    if (point.x > 0.0) {
        radius.x = radius.x;
        radius.y = radius.y;
    } else {
        radius.x = radius.z;
        radius.y = radius.w;
    }

    if (point.y > 0.0) {
        radius.x = radius.x;
    } else {
        radius.x = radius.y;
    }


    var q: vec2<f32> = abs(point)-size +radius.x;
    return min(max(q.x,q.y),0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - radius.x;
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.fragCoords = vec2<f32>(model.position.x, model.position.y);
    return out;
}



@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    var screen_dimensions = vec2<f32>(800.0, 600.0); // Needs to be a uniform
    var u_dimensions = vec2<f32>(400.0, 300.0); // Needs to be a uniform

    var point = 1.0 * in.fragCoords / 0.5; 
    var box_size = vec2<f32>(0.95, 0.95);

    var d = sd_round_box(point, box_size, vec4<f32>(0.02, 0.02, 0.02, 0.02));
    if (d > 0.0) {
        return vec4<f32>(1.0, 0.0, 0.0, 0.0);
    }

    return vec4<f32>(0.0, 1.0, 0.0, 0.0);
}
