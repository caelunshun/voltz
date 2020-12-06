#version 440

layout (location = 0) in vec2 iTexCoord;

layout (location = 0) out vec4 oColor;

layout (set = 0, binding = 0) uniform texture2D uTexture;
layout (set = 0, binding = 1) uniform sampler uSampler;

void main() {
    oColor = texture(sampler2D(uTexture, uSampler), iTexCoord);
}
