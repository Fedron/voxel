use std::collections::HashMap;

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::chunk::{Chunk, Voxel};

#[bon::builder]
#[derive(Debug, Clone)]
pub struct WorldGeneratorOptions {
    pub seed: u32,
    pub chunk_size: glam::UVec3,
    pub world_size: glam::UVec3,
    pub continent_frequency: f64,
    pub continent_lacunarity: f64,
}

pub struct WorldGenerator {
    options: WorldGeneratorOptions,
    generator: Fbm<Perlin>,
}

impl WorldGenerator {
    pub fn new(options: WorldGeneratorOptions) -> Self {
        let continent_generator = Fbm::<Perlin>::new(options.seed)
            .set_frequency(options.continent_frequency)
            .set_persistence(0.5)
            .set_lacunarity(options.continent_lacunarity)
            .set_octaves(4);

        Self {
            options,
            generator: continent_generator,
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
        let world_position = (grid_position * self.options.chunk_size).as_dvec3();

        for x in 0..self.options.chunk_size.x {
            for z in 0..self.options.chunk_size.z {
                let color = if self
                    .generator
                    .get([world_position.x + x as f64, world_position.z + z as f64])
                    > 0.0
                {
                    255u8
                } else {
                    0u8
                };

                chunk.set_voxel(
                    glam::uvec3(x, 0, z),
                    Voxel::Color([color, color, color, 255]),
                );
            }
        }

        chunk
    }
}
