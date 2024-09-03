use noise::{Cache, Curve, Min, MultiFractal, NoiseFn, Perlin, RidgedMulti, Seedable, Turbulence};

use super::WorldGeneratorOptions;

#[derive(Debug, Clone, Copy)]
pub struct RiverOptions {
    pub depth: f64,
}

impl RiverOptions {
    pub fn as_noise(&self, world: &WorldGeneratorOptions) -> impl NoiseFn<f64, 2> {
        let base = RidgedMulti::<Perlin>::new(world.seed + 80)
            .set_frequency(18.75)
            .set_lacunarity(world.continent_lacunarity)
            .set_octaves(1);

        let curve = Curve::new(base)
            .add_control_point(-2.0, 2.0)
            .add_control_point(-1.0, 1.0)
            .add_control_point(-0.125, 0.875)
            .add_control_point(0.0, -1.0)
            .add_control_point(1.0, -1.5)
            .add_control_point(2.0, -2.0);

        let base1 = RidgedMulti::<Perlin>::new(world.seed + 81)
            .set_frequency(43.25)
            .set_lacunarity(world.continent_lacunarity)
            .set_octaves(1);

        let curve1 = Curve::new(base1)
            .add_control_point(-2.0, 2.0)
            .add_control_point(-1.0, 1.5)
            .add_control_point(-0.125, 1.4375)
            .add_control_point(0.0, 0.5)
            .add_control_point(1.0, 0.25)
            .add_control_point(2.0, 0.0);

        let min = Min::new(curve, curve1);

        let tu = Turbulence::<_, Perlin>::new(min)
            .set_seed(world.seed + 82)
            .set_frequency(9.25)
            .set_power(1.0 / 57.75)
            .set_roughness(6);

        Cache::new(tu)
    }
}
