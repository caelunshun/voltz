#version 440

layout (location = 0) in vec3 v_texcoord;
layout (location = 1) in vec3 v_view_pos;
layout (location = 2) in vec3 v_world_pos;
layout (location = 3) in vec3 v_normal;

layout (location = 0) out vec4 f_color;

layout (set = 0, binding = 1) uniform texture2DArray u_block_textures;
layout (set = 0, binding = 2) uniform sampler u_block_sampler;

const vec4 fog_color = vec4(0.6, 0.7, 0.8, 1.0);
const float fog_density = 0.005;

const vec3 light_dir1 = vec3(1.0, 1.0, 0.3);
const vec3 light_dir2 = vec3(-0.4, -0.7, -0.8);
const vec3 light_color = vec3(1.0, 0.8, 0.5);

void main() {
    // Shading
    float ambient = 0.3;
    vec3 normal = normalize(v_normal);
    float diff = max(dot(normal, light_dir1), 0.0) + max(dot(normal, light_dir2), 0.0) * 0.4;
    vec4 shaded = vec4((ambient + diff) * light_color, 1.0);

    // Compute fog
    float fog_depth = length(v_view_pos);
    #define LOG2 1.442695
    float fog_amount = 1. - exp2(-fog_density * fog_density * fog_depth * fog_depth * LOG2);

    vec4 col = shaded * texture(sampler2DArray(u_block_textures, u_block_sampler), v_texcoord);

    col = mix(col, fog_color, fog_amount);

    f_color = col;
}
