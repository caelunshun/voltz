#version 440

layout (location = 0) in vec3 iPos;
layout (location = 1) in vec3 iTexCoord;
layout (location = 2) in vec3 iNormal;

layout (location = 0) out vec3 oTexCoord;
layout (location = 1) out vec3 oViewPos;
layout (location = 2) out vec3 oWorldPos;
layout (location = 3) out vec3 oNormal;

layout (push_constant) uniform Globals {
    vec4 uTransform;
    mat4 uView;
    mat4 uPerspective;
};

void main() {
    oTexCoord = iTexCoord;
    oNormal = iNormal;

    oWorldPos = (uTransform + vec4(iPos, 1.0)).xyz;

    oViewPos = (uView * vec4(oWorldPos, 1.0)).xyz;

    vec4 cameraPos = uPerspective * vec4(oViewPos, 1.0);
    gl_Position = cameraPos;
}
