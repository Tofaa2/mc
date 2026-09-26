use super::*;

pub const SEA_LEVEL: i32 = 6;
#[cfg(test)]
pub const DEFAULT_SEED: u32 = 71024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Meadow,
    Grove,
    Highlands,
}

pub struct Generator {
    pub seed: u32,
}

impl Generator {
    fn hash(&self, x: i32, y: i32, z: i32, salt: u32) -> f32 {
        let mut h = self.seed ^ salt;
        for n in [x, y, z] {
            h ^= (n as u32).wrapping_mul(0x9e3779b9);
            h = (h ^ (h >> 16)).wrapping_mul(0x85ebca6b);
        }
        (h ^ (h >> 13)) as f32 / u32::MAX as f32
    }

    fn noise(&self, x: f32, y: f32, z: f32, salt: u32) -> f32 {
        let p = Vec3::new(x, y, z);
        let cell = p.floor().as_ivec3();
        let f = p - p.floor();
        let t = f * f * (Vec3::splat(3.) - 2. * f);
        let mut value = 0.;
        for dx in 0..=1 {
            for dy in 0..=1 {
                for dz in 0..=1 {
                    let weight = (if dx == 0 { 1. - t.x } else { t.x })
                        * (if dy == 0 { 1. - t.y } else { t.y })
                        * (if dz == 0 { 1. - t.z } else { t.z });
                    value += self.hash(cell.x + dx, cell.y + dy, cell.z + dz, salt) * weight;
                }
            }
        }
        value * 2. - 1.
    }

    fn fbm(&self, x: f32, z: f32, salt: u32) -> f32 {
        let mut sum = 0.;
        let mut frequency = 1.;
        let mut amplitude = 0.57;
        for octave in 0..4 {
            sum += self.noise(x * frequency, 0.37, z * frequency, salt + octave) * amplitude;
            frequency *= 2.07;
            amplitude *= 0.5;
        }
        sum
    }

    pub fn column(&self, x: i32, z: i32) -> (i32, Biome) {
        let x = x as f32;
        let z = z as f32;
        let wx = x + self.fbm(x / 110., z / 110., 11) * 22.;
        let wz = z + self.fbm(x / 110., z / 110., 19) * 22.;
        let continental = self.fbm(wx / 95., wz / 95., 29);
        let hills = self.fbm(wx / 30., wz / 30., 41);
        let mountain = ((self.noise(wx / 72., 0.5, wz / 72., 53) + 0.25) * 1.8).clamp(0., 1.);
        let ridge = 1. - self.fbm(wx / 24., wz / 24., 67).abs();
        let land = 10. + continental * 15. + hills * 6. + mountain * ridge * 24.;
        let channel = self.noise(wx / 64., 0.5, wz / 64., 79).abs();
        let bank = ((channel - 0.035) / 0.13).clamp(0., 1.);
        let bank = bank * bank * (3. - 2. * bank);
        let height = (land.min(3.) + (land - land.min(3.)) * bank).round() as i32;
        let moisture = self.fbm(x / 45., z / 45., 97);
        let biome = if height > 24 {
            Biome::Highlands
        } else if moisture > -0.12 {
            Biome::Grove
        } else {
            Biome::Meadow
        };
        (height.clamp(WORLD_BOTTOM + 6, 48), biome)
    }

    fn cave(&self, x: i32, y: i32, z: i32, surface: i32) -> bool {
        if y <= WORLD_BOTTOM + 2 || y > surface - 3 {
            return false;
        }
        let (x, y, z) = (x as f32, y as f32, z as f32);
        // Two intersecting density fields form connected winding tunnels.
        let a = self.noise(x / 18., y / 12., z / 18., 113);
        let b = self.noise(x / 21., y / 15., z / 21., 127);
        a.abs() < 0.14 && b.abs() < 0.18
    }

    #[cfg(test)]
    pub fn generate(&self, world: &mut WorldData) {
        self.generate_region(
            world,
            IVec2::splat(-WORLD_RADIUS),
            IVec2::splat(WORLD_RADIUS),
        );
    }

    pub fn generate_region(&self, world: &mut WorldData, min: IVec2, max: IVec2) {
        world.blocks.clear();
        world.max_y = 56;
        for x in min.x..=max.x {
            for z in min.y..=max.y {
                let (height, biome) = self.column(x, z);
                let slope = (height - self.column(x + 1, z).0)
                    .abs()
                    .max((height - self.column(x, z + 1).0).abs());
                for y in WORLD_BOTTOM..=height.max(SEA_LEVEL) {
                    if y <= height && self.cave(x, y, z, height) {
                        continue;
                    }
                    let depth = height - y;
                    let kind = if y > height {
                        BlockKind::Water
                    } else if depth == 0 {
                        if height <= SEA_LEVEL + 1 {
                            BlockKind::Sand
                        } else if height >= 33 {
                            BlockKind::Snow
                        } else if slope >= 3 || biome == Biome::Highlands {
                            BlockKind::LavenderStone
                        } else {
                            BlockKind::Grass
                        }
                    } else if depth <= 3 {
                        if height <= SEA_LEVEL + 1 {
                            BlockKind::Sand
                        } else {
                            BlockKind::Dirt
                        }
                    } else if y < 0
                        && self.noise(x as f32 / 3., y as f32 / 3., z as f32 / 3., 149) > 0.65
                    {
                        BlockKind::GoldOre
                    } else if self.noise(x as f32 / 4., y as f32 / 4., z as f32 / 4., 151) > 0.63 {
                        BlockKind::RoseOre
                    } else if y < -14
                        && self.noise(x as f32 / 5., y as f32 / 5., z as f32 / 5., 157) > 0.7
                    {
                        BlockKind::Moonstone
                    } else if y < -8 {
                        BlockKind::DeepSoil
                    } else {
                        BlockKind::RoseStone
                    };
                    world.blocks.insert(IVec3::new(x, y, z), kind);
                }
            }
        }
        // Jittered cells provide deterministic spacing without rows of trees.
        for cx in min.x.div_euclid(9)..=max.x.div_euclid(9) {
            for cz in min.y.div_euclid(9)..=max.y.div_euclid(9) {
                let x = cx * 9 + 2 + (self.hash(cx, 0, cz, 163) * 5.) as i32;
                let z = cz * 9 + 2 + (self.hash(cx, 0, cz, 167) * 5.) as i32;
                if x < min.x || x > max.x || z < min.y || z > max.y {
                    continue;
                }
                let (height, biome) = self.column(x, z);
                let chance = if biome == Biome::Grove { 0.8 } else { 0.16 };
                if self.hash(cx, 0, cz, 173) > chance
                    || world.blocks.get(&IVec3::new(x, height, z)) != Some(&BlockKind::Grass)
                {
                    continue;
                }
                let trunk = 4 + (self.hash(cx, 0, cz, 179) * 3.) as i32;
                let clear = (-2..=2).all(|dx| {
                    (-2..=2).all(|dz| self.column(x + dx, z + dz).0 < height + trunk - 1)
                });
                if !clear {
                    continue;
                }
                for dy in 1..=trunk {
                    world.blocks.insert(
                        IVec3::new(x, height + dy, z),
                        if dy == trunk {
                            BlockKind::Planks
                        } else {
                            BlockKind::Wood
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_columns_are_repeatable_and_varied() {
        let a = Generator { seed: DEFAULT_SEED };
        let b = Generator { seed: DEFAULT_SEED };
        let other = Generator {
            seed: DEFAULT_SEED + 1,
        };
        let mut min = i32::MAX;
        let mut max = i32::MIN;
        let mut changed = 0;
        let mut biomes = [0; 3];
        let mut wet = 0;
        for x in -WORLD_RADIUS..=WORLD_RADIUS {
            for z in -WORLD_RADIUS..=WORLD_RADIUS {
                let col = a.column(x, z);
                assert_eq!(col, b.column(x, z));
                changed += usize::from(col != other.column(x, z));
                min = min.min(col.0);
                max = max.max(col.0);
                biomes[col.1 as usize] += 1;
                wet += usize::from(col.0 < SEA_LEVEL);
            }
        }
        eprintln!("Height range {min}..{max}; biomes {biomes:?}; wet columns {wet}");
        assert!(max - min > 20);
        assert!(changed > 1000);
        assert!(wet > 100);
        assert!(biomes.iter().all(|n| *n > 100));
    }

    #[test]
    fn generated_world_has_sealed_bottom_caves_and_level_water() {
        let generator = Generator { seed: DEFAULT_SEED };
        let mut world = WorldData::default();
        generator.generate(&mut world);
        let mut caves = 0;
        let mut water = 0;
        for x in (-WORLD_RADIUS..=WORLD_RADIUS).step_by(3) {
            for z in (-WORLD_RADIUS..=WORLD_RADIUS).step_by(3) {
                let height = generator.column(x, z).0;
                assert!(world.blocks.contains_key(&IVec3::new(x, WORLD_BOTTOM, z)));
                for y in WORLD_BOTTOM + 3..height - 3 {
                    caves += usize::from(!world.blocks.contains_key(&IVec3::new(x, y, z)));
                }
                if height < SEA_LEVEL {
                    assert!(
                        world.blocks.get(&IVec3::new(x, SEA_LEVEL, z)) == Some(&BlockKind::Water)
                    );
                    water += 1;
                }
            }
        }
        assert!(caves > 100);
        assert!(water > 10);
    }
}
