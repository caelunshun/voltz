// Vertex shader to blit a texture onto a 2D quad.
// Used for UI rendering.
//
// Does not use a vertex buffer. Just run
// with vertex_count=6 and the quad will be generated.

#version 440

layout (push_constant) uniform Globals {
    mat4 uOrtho;
    vec2 uPos;
    vec2 uSize;
};

layout (location = 0) out vec2 oTexCoord;

vec2[6] lookupTable = {
    vec2(0, 0),
    vec2(1, 0),
    vec2(1, 1),
    vec2(1, 1),
    vec2(0, 1),
    vec2(0, 0),
};

void main() {
    oTexCoord = lookupTable[gl_VertexIndex];
    gl_Position = uOrtho * vec4(oTexCoord * uSize + uPos, 0.0, 1.0);
}
