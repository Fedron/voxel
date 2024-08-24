use std::collections::HashMap;

use glam::FloatExt;
use noise::{core::perlin::perlin_2d, permutationtable::PermutationTable};

use crate::chunk::{Chunk, Voxel};

#[bon::builder]
pub struct WorldGeneratorOptions {
    seed: u32,
    chunk_size: glam::UVec3,
    world_size: glam::UVec3,
    max_terrain_height: u32,
}

pub struct WorldGenerator {
    options: WorldGeneratorOptions,
    permutation_table: PermutationTable,
}

impl WorldGenerator {
    pub fn new(options: WorldGeneratorOptions) -> Self {
        let permutation_table = PermutationTable::new(options.seed);

        Self {
            options,
            permutation_table,
        }
    }

    pub fn generate_world(&self) -> HashMap<glam::UVec3, Chunk> {
        let mut world = HashMap::new();

        for x in 0..self.options.world_size.x {
            for y in 0..self.options.world_size.y {
                for z in 0..self.options.world_size.z {
                    world.insert(
                        glam::uvec3(x, y, z),
                        self.generate_chunk(glam::uvec3(x, y, z)),
                    );
                }
            }
        }

        world
    }

    pub fn generate_chunk(&self, grid_position: glam::UVec3) -> Chunk {
        let mut chunk = Chunk::new(grid_position, self.options.chunk_size);

        for x in 0..self.options.chunk_size.x {
            for z in 0..self.options.chunk_size.z {
                let height = perlin_2d(
                    (
                        ((grid_position.x * self.options.chunk_size.x) + x) as f64 / 128.0,
                        ((grid_position.z * self.options.chunk_size.z) + z) as f64 / 128.0,
                    )
                        .into(),
                    &self.permutation_table,
                )
                .remap(-1.0, 1.0, 1.0, self.options.max_terrain_height as f64)
                .floor() as u32;

                for y in 0..self.options.chunk_size.y {
                    if (grid_position.y * self.options.chunk_size.y) + y < height {
                        chunk.set_voxel(glam::UVec3::new(x, y as u32, z), Voxel::Stone);
                    }
                }
            }
        }

        chunk
    }
}
