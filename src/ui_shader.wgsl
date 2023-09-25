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
    var screen_size = vec2<f32>(800.0, 600.0); // Needs to be a uniform
    var rect_size = vec2<f32>(400.0, 300.0); // Needs to be a uniform

    // This gives us values from -1.0 to 1.0 that are relative to the rectangle 
    // such that the top left of the rect is -1.0, -1.0 - just makes it easier
    // to run mathematical formulas on this
    var point = 1.0 * in.fragCoords / (rect_size / screen_size);  
    var box_size = vec2<f32>(0.95, 0.95);

    var d = sd_round_box(point, box_size, vec4<f32>(0.5, 0.02, 0.02, 0.02));
    if (d > 0.0) {
        return vec4<f32>(1.0, 0.0, 0.0, 0.0);
    }

    return vec4<f32>(0.0, 1.0, 0.0, 0.0);
}
