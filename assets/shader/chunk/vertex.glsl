#version 440

layout (location = 0) in vec3 a_pos;
layout (location = 1) in vec3 a_texcoord;
layout (location = 2) in vec3 a_normal;

layout (location = 0) out vec3 v_texcoord;
layout (location = 1) out vec3 v_view_pos;
layout (location = 2) out vec3 v_world_pos;
layout (location = 3) out vec3 v_normal;

layout (set = 0, binding = 0) uniform Uniforms {
    mat4 u_transform;
};
layout (push_constant) uniform PushConstants {
    mat4 u_view;
    mat4 u_perspective;
};

void main() {
    v_texcoord = a_texcoord;

    v_view_pos = (u_view * u_transform * vec4(a_pos, 1.0)).xyz;

    v_world_pos = (u_transform * vec4(a_pos, 1.0)).xyz;

    vec4 camera_pos = u_perspective * u_view * u_transform * vec4(a_pos, 1.0);
    gl_Position = camera_pos;

    v_normal = a_normal;
}
