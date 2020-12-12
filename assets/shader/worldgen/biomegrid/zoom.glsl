// Increases detail but does not
// introduce new values.
//
// Output size: 2n - 1

#version 450
#include <rng.glsl>

#define INPUT_DIM 16
#define OUTPUT_DIM 31

layout (
    local_size_x = OUTPUT_DIM,
    local_size_y = OUTPUT_DIM
) in;

layout (set = 0, binding = 0, r8ui) uniform readonly uimage2D uInputGrid;
layout (set = 0, binding = 1) uniform writeonly uimage2D uOutputGrid;

layout (push_constant) uniform PushConstants {
    uint uSeed;
    ivec2 uOffset;
};

void main() {
    ivec2 outCoords = ivec2(gl_LocalInvocationID.xy + gl_WorkGroupID.xy * OUTPUT_DIM);

    ivec2 inTopLeft = outCoords / 2;
    ivec2 inTopRight = ivec2((outCoords.x + 1) / 2, outCoords.y / 2);
    ivec2 inBottomLeft = ivec2(outCoords.x / 2, (outCoords.y + 1) / 2);
    ivec2 inBottomRight = ivec2((outCoords.x + 1) / 2, (outCoords.y + 1) / 2);

    uint topLeft = imageLoad(uInputGrid, inTopLeft).x;
    uint topRight = imageLoad(uInputGrid, inTopRight).x;
    uint bottomLeft = imageLoad(uInputGrid, inBottomLeft).x;
    uint bottomRight = imageLoad(uInputGrid, inBottomRight).x;

    ivec2 globalPos = outCoords + uOffset;
    uint rand = random(uvec2(globalPos + uSeed)) % 4;

    uint[4] values = { topLeft, topRight, bottomLeft, bottomRight };
    uint value = values[rand];

    imageStore(uOutputGrid, outCoords, uvec4(value, 0, 0, 0));
}
