// Distributes land biomes across the grid
// when parts are identified as land.
//
// Interprets 0 as ocean and 1 as land.
//
// Output size: n (unchanged)

#version 450
#include <noise.glsl>
#include <biomes.glsl>

#define DIM 32

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
    ivec2 inCoords = ivec2(gl_LocalInvocationID.xy + gl_WorkGroupID.xy * DIM);
    uint value = imageLoad(uInputGrid, inCoords).x;

    uint result;
    if (value == 0) {
        // Ocean: leave unchanged.
        result = value;
    } else {
        // Land: determine biome based on noise.
        vec2 noiseInput = (uOffset + inCoords + uSeed) * 0.05;
        float noiseValue = simplexNoise2D(noiseInput);

        if (noiseValue < -0.5) {
            result = BIOME_FOREST;
        } else if (noiseValue < -0.2) {
            result = BIOME_HILLS;
        } else if (noiseValue < 0.4) {
            result = BIOME_PLAINS;
        } else {
            result = BIOME_DESERT;
        }
    }

    imageStore(uOutputGrid, inCoords, uvec4(result, 0, 0, 0));
}
