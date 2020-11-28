#version 440

layout (location = 0) in vec3 v_texcoord;

layout (location = 0) out vec4 f_color;

layout (set = 0, binding = 1) uniform texture2DArray u_block_textures;
layout (set = 0, binding = 2) uniform sampler u_block_sampler;

void main() {
    vec4 col = texture(sampler2DArray(u_block_textures, u_block_sampler), v_texcoord);
    f_color = col;
}
