use hills::HillOptions;
use mountains::MountainOptions;
use noise::{
    Add, Cache, Clamp, Curve, Fbm, Min, MultiFractal, NoiseFn, Perlin, RidgedMulti, ScaleBias,
    Seedable, Select, Terrace, Turbulence,
};
use plains::PlainOptions;
use rivers::RiverOptions;

use crate::chunk::{Chunk, Voxel};

pub mod hills;
pub mod mountains;
pub mod plains;
pub mod rivers;

#[derive(Debug, Clone, Copy)]
pub struct WorldGeneratorOptions {
    pub seed: u32,
    pub chunk_size: glam::UVec3,

    pub continent_frequency: f64,
    pub continent_lacunarity: f64,
    pub continent_height_scale: f64,
    pub sea_level: f64,

    pub shelf_level: f64,
    pub terrain_offset: f64,

    pub mountain_options: MountainOptions,
    pub hill_options: HillOptions,
    pub plain_options: PlainOptions,
    pub river_options: RiverOptions,
}

pub struct WorldGenerator {
    pub options: WorldGeneratorOptions,
    generator: Box<dyn NoiseFn<f64, 2>>,
}

impl WorldGenerator {
    pub fn new(options: WorldGeneratorOptions) -> Self {
        let now = std::time::Instant::now();
        let continent_with_plains = || {
            let add = Add::new(
                Self::continent_elevation(&options),
                options.plain_options.as_scaled_noise(&options),
            );
            Cache::new(add)
        };

        let continent_with_hills = || {
            let add = Add::new(
                continent_with_plains(),
                options.hill_options.as_scaled_noise(&options),
            );

            let select = Select::new(
                continent_with_plains(),
                add,
                Self::terrain_definition(&options),
            )
            .set_bounds(
                1.0 - options.hill_options.amount,
                1001.0 - options.hill_options.amount,
            )
            .set_falloff(0.25);

            Cache::new(select)
        };

        let continent_with_mountains = || {
            let add = Add::new(
                Self::continent_elevation(&options),
                options.mountain_options.as_scaled_noise(&options),
            );

            let curve = Curve::new(Self::continent_definition(&options))
                .add_control_point(-1.0, -0.0625)
                .add_control_point(0.0, 0.0)
                .add_control_point(1.0 - options.mountain_options.amount, 0.0625)
                .add_control_point(1.0, 0.25);

            let add = Add::new(add, curve);

            let select = Select::new(
                continent_with_hills(),
                add,
                Self::terrain_definition(&options),
            )
            .set_bounds(
                1.0 - options.mountain_options.amount,
                1001.0 - options.mountain_options.amount,
            )
            .set_falloff(0.25);

            Cache::new(select)
        };

        let continent_with_rivers = {
            let scaled = ScaleBias::new(options.river_options.as_noise(&options))
                .set_scale(options.river_options.depth / 2.0)
                .set_bias(-options.river_options.depth / 2.0);

            let add = Add::new(continent_with_mountains(), scaled);

            let select = Select::new(
                continent_with_mountains(),
                add,
                Self::terrain_definition(&options),
            )
            .set_bounds(
                options.sea_level,
                options.continent_height_scale + options.sea_level,
            )
            .set_falloff(options.continent_height_scale - options.sea_level);

            Cache::new(select)
        };
        println!("Noise function init took {:?}", now.elapsed());

        Self {
            options,
            generator: Box::new(continent_with_rivers),
        }
    }

    pub fn generate_chunk(&self, grid_position: glam::IVec3) -> Chunk {
        let mut chunk = Chunk::new(grid_position, self.options.chunk_size);
        let world_position = (grid_position * self.options.chunk_size.as_ivec3()).as_dvec3();

        for x in 0..self.options.chunk_size.x {
            for z in 0..self.options.chunk_size.z {
                let color =
                    match self.get_height(world_position + glam::dvec3(x as f64, 0.0, z as f64)) {
                        Height::DeepOcean => [16, 23, 77],
                        Height::Ocean => [37, 104, 207],
                        Height::Shore => [51, 152, 241],
                        Height::Plain => [63, 210, 64],
                        Height::Hill => [33, 156, 34],
                        Height::Mountain => [105, 107, 109],
                        Height::Peak => [227, 233, 239],
                    };

                chunk.set_voxel(
                    glam::uvec3(x, 0, z),
                    Voxel::Color([color[0], color[1], color[2], 255]),
                );
            }
        }

        chunk
    }
}

enum Height {
    DeepOcean,
    Ocean,
    Shore,
    Plain,
    Hill,
    Mountain,
    Peak,
}

impl WorldGenerator {
    fn base_continent_definition(options: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
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
            .set_frequency(options.continent_frequency * 4.34375)
            .set_persistence(0.5)
            .set_lacunarity(options.continent_lacunarity)
            .set_octaves(11);

        let scaled = ScaleBias::new(carver).set_scale(0.375).set_bias(0.625);

        let min = Min::new(scaled, curve);
        let clamped = Clamp::new(min).set_bounds(-1.0, 1.0);

        Cache::new(clamped)
    }

    fn continent_definition(options: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let tu0 = Turbulence::<_, Perlin>::new(Self::base_continent_definition(&options))
            .set_seed(options.seed + 10)
            .set_frequency(options.continent_frequency * 15.25)
            .set_power(options.continent_frequency / 113.75)
            .set_roughness(13);

        let tu1 = Turbulence::<_, Perlin>::new(tu0)
            .set_seed(options.seed + 11)
            .set_frequency(options.continent_frequency * 47.25)
            .set_power(options.continent_frequency / 433.75)
            .set_roughness(12);

        let tu2 = Turbulence::<_, Perlin>::new(tu1)
            .set_seed(options.seed + 12)
            .set_frequency(options.continent_frequency * 95.25)
            .set_power(options.continent_frequency / 1019.75)
            .set_roughness(11);

        let select = Select::new(
            Self::base_continent_definition(&options),
            tu2,
            Self::base_continent_definition(&options),
        )
        .set_bounds(options.sea_level - 0.0375, options.sea_level + 1000.0375)
        .set_falloff(0.0625);

        Cache::new(select)
    }

    fn continent_elevation(options: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let continental_shelf = {
            let te = Terrace::new(Self::continent_definition(&options))
                .add_control_point(-1.0)
                .add_control_point(-0.75)
                .add_control_point(options.shelf_level)
                .add_control_point(1.0);

            let clamped = Clamp::new(te).set_bounds(-0.75, options.sea_level);

            let ridged_multi = RidgedMulti::<Perlin>::new(options.seed + 110)
                .set_frequency(options.continent_frequency * 4.375)
                .set_lacunarity(options.continent_lacunarity)
                .set_octaves(16);

            let scaled = ScaleBias::new(ridged_multi)
                .set_scale(-0.125)
                .set_bias(-0.125);

            let add = Add::new(scaled, clamped);

            Cache::new(add)
        };

        let continent_elevation = {
            let scaled = ScaleBias::new(Self::continent_definition(&options))
                .set_scale(options.continent_height_scale)
                .set_bias(0.0);

            let select = Select::new(
                scaled,
                continental_shelf,
                Self::continent_definition(&options),
            )
            .set_bounds(options.shelf_level - 1000.0, options.shelf_level)
            .set_falloff(0.03125);

            Cache::new(select)
        };

        continent_elevation
    }

    fn terrain_definition(options: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let tu = Turbulence::<_, Perlin>::new(Self::continent_definition(&options))
            .set_seed(options.seed + 20)
            .set_frequency(options.continent_frequency * 18.125)
            .set_power(options.continent_frequency / 20.59375 * options.terrain_offset)
            .set_roughness(3);

        let te = Terrace::new(tu)
            .add_control_point(-1.0)
            .add_control_point(options.shelf_level + options.sea_level / 2.0)
            .add_control_point(1.0);

        Cache::new(te)
    }

    fn get_height(&self, position: glam::DVec3) -> Height {
        let height = self.generator.get([position.x, position.z]);

        if height < self.options.sea_level - 0.5 {
            Height::DeepOcean
        } else if height < self.options.sea_level {
            Height::Ocean
        } else if height < self.options.sea_level + 0.125 {
            Height::Shore
        } else if height < self.options.sea_level + 0.25 {
            Height::Plain
        } else if height < self.options.sea_level + 0.5 {
            Height::Hill
        } else if height < self.options.sea_level + 0.75 {
            Height::Mountain
        } else {
            Height::Peak
        }
    }
}
