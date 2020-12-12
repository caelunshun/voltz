// Smooths the input, used to prevent noisy edges
// caused by the zoom shader.
//
// Output size: n - 2 (edges removed)

#version 450
#include <rng.glsl>

#define DIM 16

layout (
    local_size_x = DIM,
    local_size_y = DIM
) in;

layout (set = 0, binding = 0, r8ui) uniform readonly uimage2D uInputGrid;
layout (set = 0, binding = 1) uniform writeonly uimage2D uOutputGrid;

layout (push_constant) uniform PushConstants {
    uint uSeed;
    ivec2 uOffset;
};

void main() {
    ivec2 inCoords = ivec2(gl_LocalInvocationID.xy + gl_WorkGroupID.xy * DIM) + ivec2(1, 1);

    ivec2 leftCoord = inCoords - ivec2(1, 0);
    ivec2 rightCoord = inCoords + ivec2(1, 0);
    ivec2 topCoord = inCoords - ivec2(0, 1);
    ivec2 bottomCord = inCoords + ivec2(0, 1);

    uint left = imageLoad(uInputGrid, leftCoord).x;
    uint right = imageLoad(uInputGrid, rightCoord).x;
    uint top = imageLoad(uInputGrid, topCoord).x;
    uint bottom = imageLoad(uInputGrid, bottomCord).x;

    bool horizontal = left == right;
    bool vertical = top == bottom;

    uint result;
    if (horizontal && vertical) {
        uint random = random(uvec2(inCoords) + uOffset + uSeed);
        uint[2] values = { left, top };
        result = values[random % 2];
    } else if (horizontal) {
        result = left;
    } else if (vertical) {
        result = top;
    } else {
        result = imageLoad(uInputGrid, inCoords).x;
    }

    ivec2 outCoords = inCoords - ivec2(1, 1);
    imageStore(uOutputGrid, outCoords, uvec4(result, 0, 0, 0));
}
