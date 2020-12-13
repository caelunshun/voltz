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
    0.01, // hills
    0.01, // desert
    0.03, // forest
};

const float[NUM_BIOMES] cBiomeAmplitudes = {
    1.0, // ocean
    0.1, // plains
    0.15, // hills
    0.2, // desert
    0.15, // forest
};

const float[NUM_BIOMES] cBiomeMidpoints = {
    64.0, // ocean
    64.0, // plains
    90.0, // hills
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

shared float[225] amplitudeSamples;
shared float[225] midpointSamples;
shared float[225] weights;

void main() {
    uvec3 pos = gl_GlobalInvocationID;

    uint id = gl_LocalInvocationID.y;
    if (id < 225) {
        ivec2 offset = ivec2(id / 15, id % 15) - ivec2(7, 7);
        float weight = 10 / (length(vec2(offset)) + 1);
        uint biomeSample = imageLoad(uBiomeGrid, ivec2(pos.xz) + offset).x;

        amplitudeSamples[id] = cBiomeAmplitudes[biomeSample] * weight;
        midpointSamples[id] = cBiomeMidpoints[biomeSample] * weight;
        weights[id] = weight;

        if (offset == ivec2(0, 0)) {
            biome = biomeSample;
            biomeBlock = cBiomeBlocks[biomeSample];
            frequency = cBiomeFrequencies[biomeSample];
        }
    }
    barrier();
    if (id == 0) {
        float amp = 0;
        float mid = 0;
        float weightSum = 0;
        for (int i = 0; i < 225; i++) {
            amp += amplitudeSamples[i];
            mid += midpointSamples[i];
            weightSum += weights[i];
        }
        amplitude = amp / weightSum;
        midpoint = mid / weightSum;
    }
    barrier();

    float noiseValue = fbm3D(pos * frequency, 3, 2.0, 0.5) + 1;

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
