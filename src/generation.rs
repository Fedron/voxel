use glam::FloatExt;
use noise::{
    Add, Cache, Clamp, Curve, Fbm, Min, MultiFractal, NoiseFn, Perlin, RidgedMulti, ScaleBias,
    Seedable, Select, Terrace, Turbulence,
};

use crate::chunk::{Chunk, Voxel};

pub mod hills;
pub mod mountains;
pub mod plains;
pub mod rivers;

/// Defines options that control the creation of a noise module for world generation.
#[derive(Debug, Clone, Copy)]
pub struct WorldGenerationOptions {
    /// Base seed for noise modules.
    pub seed: u32,
    /// Size of each chunk in voxels.
    pub chunk_size: glam::UVec3,
    /// Maximum height of the world, in voxels.
    pub max_height: u32,
    /// Thickness of the dirt layer, in voxels.
    pub dirt_layer_thickness: u32,

    /// Frequency of generated continents.
    pub continent_frequency: f64,
    /// Lacunarity of generated continents.
    pub continent_lacunarity: f64,
    /// Scaling to apply to the base continent elevation.
    pub continent_height_scale: f64,
    /// Sea level of the world.
    pub sea_level: f64,

    /// Elevation of the continental shelf. Must be lower than `sea_level`.
    pub shelf_level: f64,
    /// Offset to apply to the terrain definition. Low values cause rough terrain to appear at higher elevations.
    pub terrain_offset: f64,

    /// Options for generating mountains.
    pub mountain_options: mountains::MountainOptions,
    /// Options for generating hills.
    pub hill_options: hills::HillOptions,
    /// Options for generating plains.
    pub plain_options: plains::PlainOptions,
    /// Options for generating rivers.
    pub river_options: rivers::RiverOptions,
}

impl WorldGenerationOptions {
    /// Creates a noise function that can be used to generate a world.
    pub fn as_noise_module(&self) -> impl NoiseFn<f64, 2> {
        let continent_with_plains = || {
            let add = Add::new(
                self.continent_elevation(),
                self.plain_options.as_noise_module(&self),
            );
            Cache::new(add)
        };

        let continent_with_hills = || {
            let add = Add::new(
                continent_with_plains(),
                self.hill_options.as_noise_module(&self),
            );

            let select = Select::new(continent_with_plains(), add, self.terrain_definition())
                .set_bounds(
                    1.0 - self.hill_options.amount,
                    1001.0 - self.hill_options.amount,
                )
                .set_falloff(0.25);

            Cache::new(select)
        };

        let continent_with_mountains = || {
            let add = Add::new(
                self.continent_elevation(),
                self.mountain_options.as_noise_module(&self),
            );

            let curve = Curve::new(self.continent_definition())
                .add_control_point(-1.0, -0.0625)
                .add_control_point(0.0, 0.0)
                .add_control_point(1.0 - self.mountain_options.amount, 0.0625)
                .add_control_point(1.0, 0.25);

            let add = Add::new(add, curve);

            let select = Select::new(continent_with_hills(), add, self.terrain_definition())
                .set_bounds(
                    1.0 - self.mountain_options.amount,
                    1001.0 - self.mountain_options.amount,
                )
                .set_falloff(0.25);

            Cache::new(select)
        };

        let continent_with_rivers = {
            let scaled = ScaleBias::new(self.river_options.as_noise_module(&self))
                .set_scale(self.river_options.depth / 2.0)
                .set_bias(-self.river_options.depth / 2.0);

            let add = Add::new(continent_with_mountains(), scaled);

            let select = Select::new(continent_with_mountains(), add, self.terrain_definition())
                .set_bounds(self.sea_level, self.continent_height_scale + self.sea_level)
                .set_falloff(self.continent_height_scale - self.sea_level);

            Cache::new(select)
        };

        continent_with_rivers
    }

    fn base_continent_definition(&self) -> impl NoiseFn<f64, 2> {
        let base = Fbm::<Perlin>::new(self.seed)
            .set_frequency(self.continent_frequency)
            .set_persistence(0.5)
            .set_lacunarity(self.continent_lacunarity)
            .set_octaves(4);

        let curve = Curve::new(base)
            .add_control_point(-2.0 + self.sea_level, -1.625 + self.sea_level)
            .add_control_point(-1.0000 + self.sea_level, -1.375 + self.sea_level)
            .add_control_point(0.0000 + self.sea_level, -0.375 + self.sea_level)
            .add_control_point(0.0625 + self.sea_level, 0.125 + self.sea_level)
            .add_control_point(0.1250 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(0.2500 + self.sea_level, 1.000 + self.sea_level)
            .add_control_point(0.5000 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(0.7500 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(1.0000 + self.sea_level, 0.500 + self.sea_level)
            .add_control_point(2.0000 + self.sea_level, 0.500 + self.sea_level);

        let carver = Fbm::<Perlin>::new(self.seed + 1)
            .set_frequency(self.continent_frequency * 4.34375)
            .set_persistence(0.5)
            .set_lacunarity(self.continent_lacunarity)
            .set_octaves(11);

        let scaled = ScaleBias::new(carver).set_scale(0.375).set_bias(0.625);

        let min = Min::new(scaled, curve);
        let clamped = Clamp::new(min).set_bounds(-1.0, 1.0);

        Cache::new(clamped)
    }

    fn continent_definition(&self) -> impl NoiseFn<f64, 2> {
        let tu0 = Turbulence::<_, Perlin>::new(Self::base_continent_definition(&self))
            .set_seed(self.seed + 10)
            .set_frequency(self.continent_frequency * 15.25)
            .set_power(self.continent_frequency / 113.75)
            .set_roughness(13);

        let tu1 = Turbulence::<_, Perlin>::new(tu0)
            .set_seed(self.seed + 11)
            .set_frequency(self.continent_frequency * 47.25)
            .set_power(self.continent_frequency / 433.75)
            .set_roughness(12);

        let tu2 = Turbulence::<_, Perlin>::new(tu1)
            .set_seed(self.seed + 12)
            .set_frequency(self.continent_frequency * 95.25)
            .set_power(self.continent_frequency / 1019.75)
            .set_roughness(11);

        let select = Select::new(
            Self::base_continent_definition(&self),
            tu2,
            Self::base_continent_definition(&self),
        )
        .set_bounds(self.sea_level - 0.0375, self.sea_level + 1000.0375)
        .set_falloff(0.0625);

        Cache::new(select)
    }

    fn continent_elevation(&self) -> impl NoiseFn<f64, 2> {
        let continental_shelf = {
            let te = Terrace::new(Self::continent_definition(&self))
                .add_control_point(-1.0)
                .add_control_point(-0.75)
                .add_control_point(self.shelf_level)
                .add_control_point(1.0);

            let clamped = Clamp::new(te).set_bounds(-0.75, self.sea_level);

            let ridged_multi = RidgedMulti::<Perlin>::new(self.seed + 110)
                .set_frequency(self.continent_frequency * 4.375)
                .set_lacunarity(self.continent_lacunarity)
                .set_octaves(16);

            let scaled = ScaleBias::new(ridged_multi)
                .set_scale(-0.125)
                .set_bias(-0.125);

            let add = Add::new(scaled, clamped);

            Cache::new(add)
        };

        let continent_elevation = {
            let scaled = ScaleBias::new(Self::continent_definition(&self))
                .set_scale(self.continent_height_scale)
                .set_bias(0.0);

            let select = Select::new(scaled, continental_shelf, Self::continent_definition(&self))
                .set_bounds(self.shelf_level - 1000.0, self.shelf_level)
                .set_falloff(0.03125);

            Cache::new(select)
        };

        continent_elevation
    }

    fn terrain_definition(&self) -> impl NoiseFn<f64, 2> {
        let tu = Turbulence::<_, Perlin>::new(Self::continent_definition(&self))
            .set_seed(self.seed + 20)
            .set_frequency(self.continent_frequency * 18.125)
            .set_power(self.continent_frequency / 20.59375 * self.terrain_offset)
            .set_roughness(3);

        let te = Terrace::new(tu)
            .add_control_point(-1.0)
            .add_control_point(self.shelf_level + self.sea_level / 2.0)
            .add_control_point(1.0);

        Cache::new(te)
    }
}

impl WorldGenerationOptions {
    /// Returns the height of the sea level in voxels.
    pub fn sea_level_voxels(&self) -> i32 {
        self.sea_level
            .remap(-1.0, 1.0, 0.0, self.max_height as f64)
            .floor() as i32
    }
}

/// Generates a chunk of voxels using the given world generation options.
pub fn generate_chunk(options: WorldGenerationOptions, grid_position: glam::IVec3) -> Chunk {
    let noise_module = options.as_noise_module();

    let mut chunk = Chunk::new(grid_position, options.chunk_size);
    let world_position = (grid_position * options.chunk_size.as_ivec3()).as_dvec3();

    for x in 0..options.chunk_size.x {
        for z in 0..options.chunk_size.z {
            let position = world_position + glam::dvec3(x as f64, 0.0, z as f64);
            let terrain_height = noise_module
                .get([position.x, position.z])
                .remap(-1.0, 1.0, 0.0, options.max_height as f64)
                .floor() as i32;

            for y in 0..options.chunk_size.y as i32 {
                let global_y = options.chunk_size.y as i32 * grid_position.y + y;
                let position = glam::uvec3(x, y as u32, z);

                if global_y == terrain_height {
                    chunk.set_voxel(
                        position,
                        if global_y <= options.sea_level_voxels() {
                            Voxel::Sand
                        } else {
                            Voxel::Grass
                        },
                    );
                } else if global_y
                    >= terrain_height.saturating_sub(options.dirt_layer_thickness as i32)
                    && global_y < terrain_height
                {
                    chunk.set_voxel(
                        position,
                        if global_y <= options.sea_level_voxels() {
                            Voxel::Sand
                        } else {
                            Voxel::Dirt
                        },
                    )
                } else if global_y < terrain_height {
                    chunk.set_voxel(position, Voxel::Stone)
                } else if global_y <= options.sea_level_voxels() {
                    chunk.set_voxel(position, Voxel::Water)
                }
            }
        }
    }

    chunk
}
