use std::collections::HashMap;

use noise::{Cache, Clamp, Curve, Fbm, Min, MultiFractal, NoiseFn, Perlin, ScaleBias};

use crate::chunk::{Chunk, Voxel};

#[derive(Debug, Clone)]
pub struct WorldGeneratorOptions {
    pub seed: u32,
    pub chunk_size: glam::UVec3,
    pub world_size: glam::UVec3,

    pub continent_frequency: f64,
    pub continent_lacunarity: f64,
    pub mountain_frequency: f64,
    pub sea_level: f64,
}

enum Continent {
    Ocean,
    Land,
}

pub struct WorldGenerator {
    options: WorldGeneratorOptions,
    continent_generator: Box<dyn NoiseFn<f64, 2>>,
}

impl WorldGenerator {
    pub fn new(options: WorldGeneratorOptions) -> Self {
        let continent_generator = {
            let base = Fbm::<Perlin>::new(options.seed)
                .set_frequency(options.continent_frequency)
                .set_persistence(0.5)
                .set_lacunarity(options.continent_lacunarity)
                .set_octaves(4);

            let curve = Curve::new(base)
                .add_control_point(-2.0 + options.sea_level, -1.625 + options.sea_level)
                .add_control_point(-1.0000 + options.sea_level, -1.375 + options.sea_level)
                .add_control_point(0.0000 + options.sea_level, -0.375 + options.sea_level)
                .add_control_point(0.0625 + options.sea_level, 0.125 + options.sea_level)
                .add_control_point(0.1250 + options.sea_level, 0.250 + options.sea_level)
                .add_control_point(0.2500 + options.sea_level, 1.000 + options.sea_level)
                .add_control_point(0.5000 + options.sea_level, 0.250 + options.sea_level)
                .add_control_point(0.7500 + options.sea_level, 0.250 + options.sea_level)
                .add_control_point(1.0000 + options.sea_level, 0.500 + options.sea_level)
                .add_control_point(2.0000 + options.sea_level, 0.500 + options.sea_level);

            let carver = Fbm::<Perlin>::new(options.seed + 1)
                .set_frequency(options.continent_frequency * options.mountain_frequency)
                .set_persistence(0.5)
                .set_lacunarity(options.continent_lacunarity)
                .set_octaves(11);

            let scaled = ScaleBias::new(carver).set_scale(0.375).set_bias(0.625);

            let min = Min::new(scaled, curve);
            let clamped = Clamp::new(min).set_bounds(-1.0, 1.0);

            Box::new(Cache::new(clamped))
        };

        Self {
            options,
            continent_generator,
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
                let color = match self
                    .get_continent(world_position + glam::dvec3(x as f64, 0.0, z as f64))
                {
                    Continent::Land => 0,
                    Continent::Ocean => 255,
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

impl WorldGenerator {
    fn get_continent(&self, position: glam::DVec3) -> Continent {
        if self.continent_generator.get([position.x, position.z]) > self.options.sea_level {
            Continent::Land
        } else {
            Continent::Ocean
        }
    }
}
