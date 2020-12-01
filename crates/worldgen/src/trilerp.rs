#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Extent {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Extent {
    /// Indexes into a 3D array with this extent.
    pub fn index(self, x: u32, y: u32, z: u32) -> usize {
        (y * (self.width * self.depth) + z * self.width + x) as usize
    }
}

/// Performs trilinear interpolation on a 3D grid,
/// outputting a new grid. The ratio between the
/// dimensions of the outputs and the inputs must be an integer.
///
/// # Panics
/// Panics if `output` does
/// not have a length large enough the fit the resulting grid.
pub fn trilerp(input: &[f32], input_size: Extent, output: &mut [f32], size_ratio: u32) {
    trilerp_naive(input, input_size, output, size_ratio);
}

/// Trilerps without SIMD.
fn trilerp_naive(input: &[f32], input_size: Extent, output: &mut [f32], size_ratio: u32) {
    let output_size = Extent {
        width: input_size.width * size_ratio,
        height: input_size.height * size_ratio,
        depth: input_size.depth * size_ratio,
    };
    for ix in 0..input_size.width - 1 {
        for iy in 0..input_size.height - 1 {
            for iz in 0..input_size.depth - 1 {
                // Bottom face
                let x0y0z0 = input[input_size.index(ix, iy, iz)];
                let x1y0z0 = input[input_size.index(ix + 1, iy, iz)];
                let x1y0z1 = input[input_size.index(ix + 1, iy, iz + 1)];
                let x0y0z1 = input[input_size.index(ix, iy, iz + 1)];

                // Top face
                let x0y1z0 = input[input_size.index(ix, iy + 1, iz)];
                let x1y1z0 = input[input_size.index(ix + 1, iy + 1, iz)];
                let x1y1z1 = input[input_size.index(ix + 1, iy + 1, iz + 1)];
                let x0y1z1 = input[input_size.index(ix, iy + 1, iz + 1)];

                for dx in 0..=size_ratio {
                    for dy in 0..=size_ratio {
                        for dz in 0..=size_ratio {
                            let rx = dx as f32 / size_ratio as f32;
                            let ry = dy as f32 / size_ratio as f32;
                            let rz = dz as f32 / size_ratio as f32;

                            // Interpolate between the eight corner points using the
                            // ratios above.
                            let x0_y0 = x0y0z0 * rx + x0y0z1 * (1. - rx);
                            let x1_y0 = x1y0z0 * rx + x1y0z1 * (1. - rx);
                            let y0 = x0_y0 * rz + x1_y0 * (1. - rz);

                            let x0_y1 = x0y1z0 * rx + x0y1z1 * (1. - rx);
                            let x1_y1 = x1y1z0 * rx + x1y1z1 * (1. - rx);
                            let y1 = x0_y1 * rz + x1_y1 * (1. - rz);

                            let result = y0 * ry + y1 * (1. - ry);

                            let tx = ix * size_ratio + dx;
                            let ty = iy * size_ratio + dy;
                            let tz = iz * size_ratio + 1 + dz;
                            output[output_size.index(tx, ty, tz)] = result;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let input = [
            // Bottom
            0.0, 0.0, 0.0, 0.0, // Top
            1.0, 1.0, 1.0, 1.0,
        ];
        let input_size = Extent {
            width: 2,
            height: 2,
            depth: 2,
        };
        let mut output = [100.0; 64];

        trilerp(&input, input_size, &mut output, 2);

        dbg!(output);
    }
}
