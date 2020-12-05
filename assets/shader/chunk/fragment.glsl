#version 440

layout (location = 0) in vec3 iTexCoord;
layout (location = 1) in vec3 iViewPos;
layout (location = 2) in vec3 iWorldPos;
layout (location = 3) in vec3 iNormal;

layout (location = 0) out vec4 oColor;

layout (set = 0, binding = 0) uniform texture2DArray uBlockTextures;
layout (set = 0, binding = 1) uniform sampler uBlockSampler;

const vec4 fogColor = vec4(0.6, 0.7, 0.8, 1.0);
const float fogDensity = 0.005;

const vec3 lightDir1 = vec3(1.0, 1.0, 0.3);
const vec3 lightDir2 = vec3(-0.4, -0.7, -0.8);
const vec3 lightColor = vec3(1.0, 0.8, 0.5);

void main() {
    // Shading
    float ambient = 0.3;
    vec3 normal = normalize(iNormal);
    float diff = max(dot(normal, lightDir1), 0.0) + max(dot(normal, lightDir2), 0.0) * 0.4;
    vec4 shaded = vec4((ambient + diff) * lightColor, 1.0);

    // Fog
    float fogDepth = length(iViewPos);
    #define LOG2 1.442695
    float fogAmount = 1. - exp2(-fogDensity * fogDensity * fogDepth * fogDepth * LOG2);

    vec4 col = shaded * texture(sampler2DArray(uBlockTextures, uBlockSampler), iTexCoord);

    col = mix(col, fogColor, fogAmount);

    oColor = col;
}
