// Adds rivers in between two biomes.
//
// Output size: n - 2

#version 450

#include <biomes.glsl>

#define DIM 16

layout (
    local_size_x = DIM,
    local_size_y = DIM
) in;

layout (set = 0, binding = 0, r8ui) uniform readonly uimage2D uInputGrid;
layout (set = 0, binding = 1, r8ui) uniform writeonly uimage2D uOutputGrid;

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

    bool river = false;
    if (left != right && left != BIOME_OCEAN && right != BIOME_OCEAN) {
        river = true;
    } else if (top != bottom && top != BIOME_OCEAN && bottom != BIOME_OCEAN) {
        river = true;
    }

    uint result;
    if (river) {
        result = BIOME_RIVER;
    } else {
        result = imageLoad(uInputGrid, inCoords).x;
    }

    ivec2 outCoords = inCoords - ivec2(1, 1);
    imageStore(uOutputGrid, outCoords, uvec4(result, 0, 0, 0));
}
