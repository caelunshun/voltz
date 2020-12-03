#!/bin/bash

shaders=("chunk")

for shader in ${shaders[@]}; do
  rm -r assets/shader_compiled/${shader} || true
  mkdir -p assets/shader_compiled/${shader}
  glslc -fshader-stage=vertex assets/shader/${shader}/vertex.glsl -o assets/shader_compiled/${shader}/vertex.spv
  glslc -fshader-stage=fragment assets/shader/${shader}/fragment.glsl -o assets/shader_compiled/${shader}/fragment.spv
done
