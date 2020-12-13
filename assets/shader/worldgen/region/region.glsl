#version 450
#include <noise.glsl>
#include <blocks.glsl>
#include <biomes.glsl>

#define REGION_DIM 256

#extension GL_EXT_shader_8bit_storage : enable

layout (
    local_size_x = 1,
    local_size_y = REGION_DIM
) in;

layout (set = 0, binding = 0) writeonly restrict buffer Output {
    uint8_t uBlocks[];
};

layout (set = 0, binding = 1, r8ui) uniform readonly restrict uimage2D uBiomeGrid;

const float[NUM_BIOMES] cBiomeFrequencies = {
    0.0, // ocean
    0.02, // plains
    0.04, // hills
    0.01, // desert
    0.03, // forest
};

const float[NUM_BIOMES] cBiomeAmplitudes = {
    1.0, // ocean
    1.2, // plains
    1.3, // hills
    1.1, // desert
    1.25, // forest
};

const float[NUM_BIOMES] cBiomeMidpoints = {
    64.0, // ocean
    64.0, // plains
    70.0, // hills
    65.0, // desert
    66.0, // forest
};

const uint[NUM_BIOMES] cBiomeBlocks = {
    BLOCK_WATER, // ocean
    BLOCK_GRASS, // plains
    BLOCK_MELIUM, // hills
    BLOCK_SAND, // desert
    BLOCK_STONE, // forest
};

shared uint biome;
shared float frequency;
shared float amplitude;
shared float midpoint;
shared uint biomeBlock;

void main() {
    uvec3 pos = gl_GlobalInvocationID;

    if (gl_WorkGroupID.y == 0) {
        biome = imageLoad(uBiomeGrid, ivec2(pos.xz)).x;
        frequency = cBiomeFrequencies[biome];
        amplitude = cBiomeAmplitudes[biome];
        midpoint = cBiomeMidpoints[biome];
        biomeBlock = cBiomeBlocks[biome];
    }
    barrier();

    float noiseValue = simplexNoise3D(pos * frequency);

    float gradient = (pos.y - midpoint) * amplitude;

    if (gradient < 0.0) {
        gradient *= 4.0;
    }

    float density = noiseValue + gradient;

    uint block;
    if (density < 0.0) {
        block = biomeBlock;
    } else {
        block = BLOCK_AIR;
    }

    uBlocks[pos.x * REGION_DIM * REGION_DIM + pos.z * REGION_DIM + pos.y] = uint8_t(block);
}
