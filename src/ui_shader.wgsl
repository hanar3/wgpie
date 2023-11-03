struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>
}

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
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

@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;


@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.clip_position = proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}


@group(0) @binding(0)
var<uniform> screen_size: vec2<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    return vec4<f32>(in.color, 1.0);
}
