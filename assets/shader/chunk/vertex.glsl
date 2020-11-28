#version 440

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_texcoord;

layout (location = 10) out vec3 v_texcoord;

layout (set = 0, binding = 0) uniform Uniforms {
    mat4 u_transform;
};
layout (push_constant) uniform PushConstants {
    mat4 u_view_perspective;
};

void main() {
    v_texcoord = a_texcoord;
    gl_Position =  u_view_perspective * u_transform * vec4(a_pos, 1.0);
}
