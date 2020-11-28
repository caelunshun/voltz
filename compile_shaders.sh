#!/bin/bash

shaders=("chunk")

for shader in ${shaders[@]}; do
  glslc -fshader-stage=vertex assets/shader/${shader}/vertex.glsl -o assets/shader_compiled/${shader}/vertex.spv
  glslc -fshader-stage=fragment assets/shader/${shader}/fragment.glsl -o assets/shader_compiled/${shader}/fragment.spv
done
