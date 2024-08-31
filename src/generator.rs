use std::collections::HashMap;

use glam::FloatExt;
use noise::{core::perlin::perlin_2d, permutationtable::PermutationTable};

use crate::chunk::{Chunk, Voxel};

#[bon::builder]
#[derive(Debug, Clone)]
pub struct WorldGeneratorOptions {
    pub seed: u32,
    pub chunk_size: glam::UVec3,
    pub world_size: glam::UVec3,
    pub max_terrain_height: u32,
    pub terrain_smoothness: f64,
    pub dirt_layer_thickness: u32,
    pub sea_level: u32,
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
        let world_position = grid_position * self.options.chunk_size;

        for x in 0..self.options.chunk_size.x {
            for z in 0..self.options.chunk_size.z {
                let terrain_height = perlin_2d(
                    (
                        (world_position.x + x) as f64 / self.options.terrain_smoothness,
                        (world_position.z + z) as f64 / self.options.terrain_smoothness,
                    )
                        .into(),
                    &self.permutation_table,
                )
                .remap(-1.0, 1.0, 1.0, self.options.max_terrain_height as f64)
                .floor() as u32;

                for y in 0..self.options.chunk_size.y {
                    let voxel_world_height = world_position.y + y;

                    if voxel_world_height == terrain_height {
                        chunk.set_voxel(
                            glam::uvec3(x, y, z),
                            if voxel_world_height <= self.options.sea_level {
                                Voxel::Sand
                            } else {
                                Voxel::Grass
                            },
                        );
                    } else if voxel_world_height
                        >= terrain_height.saturating_sub(self.options.dirt_layer_thickness)
                        && voxel_world_height < terrain_height
                    {
                        chunk.set_voxel(
                            glam::UVec3::new(x, y, z),
                            if voxel_world_height <= self.options.sea_level {
                                Voxel::Sand
                            } else {
                                Voxel::Dirt
                            },
                        );
                    } else if voxel_world_height < terrain_height {
                        chunk.set_voxel(glam::UVec3::new(x, y, z), Voxel::Stone);
                    } else if voxel_world_height < self.options.sea_level {
                        chunk.set_voxel(glam::UVec3::new(x, y, z), Voxel::Water);
                    }
                }
            }
        }

        chunk
    }
}
