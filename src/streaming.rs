use super::*;
use std::collections::HashSet;

#[derive(Component)]
pub struct Decoration(pub IVec2);

#[derive(Resource)]
pub struct Streaming {
    pub loaded: HashSet<IVec2>,
    pub edits: HashMap<IVec3, Option<BlockKind>>,
    dirty: HashSet<IVec2>,
}

impl Default for Streaming {
    fn default() -> Self {
        Self {
            loaded: (0..=2)
                .flat_map(|x| (0..=2).map(move |z| IVec2::new(x, z)))
                .collect(),
            edits: HashMap::new(),
            dirty: HashSet::new(),
        }
    }
}

pub fn chunk_of(p: IVec3) -> IVec2 {
    IVec2::new(p.x.div_euclid(16), p.z.div_euclid(16))
}

fn distance(a: IVec2, b: IVec2) -> i32 {
    (a - b).abs().max_element()
}

fn neighbors(p: IVec2) -> [IVec2; 5] {
    [p, p + IVec2::X, p - IVec2::X, p + IVec2::Y, p - IVec2::Y]
}

fn apply_edits(world: &mut WorldData, edits: &HashMap<IVec3, Option<BlockKind>>, chunk: IVec2) {
    for (&p, &kind) in edits {
        if chunk_of(p) != chunk {
            continue;
        }
        if let Some(kind) = kind {
            world.blocks.insert(p, kind);
            world.max_y = world.max_y.max(p.y);
        } else {
            world.blocks.remove(&p);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut state: ResMut<Streaming>,
    mut world: ResMut<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    foliage: Res<FoliageAssets>,
    mut water_materials: ResMut<Assets<water::WaterMaterial>>,
    atlas: Res<AtlasMaterial>,
    chunks: Query<(Entity, &WorldChunk)>,
    decorations: Query<(Entity, &Decoration)>,
    settings: Option<Res<settings::Settings>>,
    mut settled: Local<Option<(IVec2, i32)>>,
) {
    let load_radius = settings.map_or(4, |s| (s.fog_end / 16.).ceil() as i32);
    let keep_radius = load_radius + 1;
    let Ok(player) = player.get_single() else {
        return;
    };
    let center = chunk_of(player.translation.floor().as_ivec3());
    // No allocations, chunk scans, or resource writes when the resident set
    // is complete and the player has not crossed a boundary.
    if *settled == Some((center, load_radius)) && state.dirty.is_empty() {
        return;
    }
    *settled = None;
    let removed: HashSet<_> = state
        .loaded
        .iter()
        .copied()
        .filter(|p| distance(*p, center) > keep_radius)
        .collect();
    if !removed.is_empty() {
        for (entity, chunk) in &chunks {
            if removed.contains(&chunk.0) {
                commands.entity(entity).despawn_recursive();
            }
        }
        for (entity, chunk) in &decorations {
            if removed.contains(&chunk.0) {
                commands.entity(entity).despawn_recursive();
            }
        }
        world.blocks.retain(|p, _| !removed.contains(&chunk_of(*p)));
        for p in &removed {
            state.loaded.remove(p);
            state.dirty.extend(neighbors(*p));
        }
    }
    // One generation job per frame; nearest chunks first. A wider unload
    // radius prevents thrashing when crossing a chunk boundary repeatedly.
    let next = (-load_radius..=load_radius)
        .flat_map(|x| (-load_radius..=load_radius).map(move |z| center + IVec2::new(x, z)))
        .filter(|p| !state.loaded.contains(p))
        .min_by_key(|p| (*p - center).length_squared());
    if let Some(chunk) = next {
        let mut data = WorldData::default();
        terrain::Generator { seed: world_seed() }.generate_region(
            &mut data,
            chunk * 16,
            chunk * 16 + IVec2::splat(15),
        );
        apply_edits(&mut data, &state.edits, chunk);
        spawn_foliage(&mut commands, &foliage, &data);
        water::spawn(&mut commands, &mut meshes, &mut water_materials, &data);
        world.max_y = world.max_y.max(data.max_y);
        world.blocks.extend(data.blocks);
        state.loaded.insert(chunk);
        state.dirty.extend(neighbors(chunk));
        commands.spawn((
            Mesh3d(meshes.add(build_chunk_mesh(&world, chunk))),
            MeshMaterial3d(atlas.0.clone()),
            Transform::IDENTITY,
            WorldChunk(chunk),
        ));
        state.dirty.remove(&chunk);
    }
    let loaded = state.loaded.clone();
    state.dirty.retain(|p| loaded.contains(p));
    // Bound neighbor seam rebuilding too, rather than remeshing the world.
    let mut dirty: Vec<_> = state.dirty.iter().copied().collect();
    dirty.sort_by_key(|p| (*p - center).length_squared());
    for chunk in dirty.into_iter().take(2) {
        for (entity, old) in &chunks {
            if old.0 == chunk && !removed.contains(&chunk) {
                commands
                    .entity(entity)
                    .insert(Mesh3d(meshes.add(build_chunk_mesh(&world, chunk))));
            }
        }
        state.dirty.remove(&chunk);
    }
    if next.is_none() && state.dirty.is_empty() {
        *settled = Some((center, load_radius));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn travel_unloads_entities_and_data_then_restores_edits() {
        let mut app = App::new();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let foliage = create_foliage_assets(&mut meshes, &mut materials);
        let atlas = materials.add(StandardMaterial::default());
        app.insert_resource(meshes)
            .insert_resource(materials)
            .insert_resource(foliage)
            .insert_resource(Assets::<water::WaterMaterial>::default())
            .insert_resource(AtlasMaterial(atlas))
            .insert_resource(WorldData::default())
            .insert_resource(Streaming {
                loaded: HashSet::new(),
                edits: HashMap::new(),
                dirty: HashSet::new(),
            })
            .add_systems(Update, update);
        let player = app.world_mut().spawn((Player, Transform::IDENTITY)).id();
        app.update();
        let edited = IVec3::new(1, 60, 1);
        app.world_mut()
            .resource_mut::<Streaming>()
            .edits
            .insert(edited, Some(BlockKind::Crystal));
        app.world_mut()
            .entity_mut(player)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .x = 1024.;
        app.update();
        assert_eq!(app.world().resource::<Streaming>().loaded.len(), 1);
        assert!(
            app.world()
                .resource::<WorldData>()
                .blocks
                .keys()
                .all(|p| chunk_of(*p) == IVec2::new(64, 0))
        );
        assert_eq!(
            app.world_mut()
                .query::<&WorldChunk>()
                .iter(app.world())
                .count(),
            1
        );
        assert!(
            app.world_mut()
                .query::<&Decoration>()
                .iter(app.world())
                .all(|d| d.0 == IVec2::new(64, 0))
        );
        app.world_mut()
            .entity_mut(player)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .x = 0.;
        app.update();
        assert!(
            app.world().resource::<WorldData>().blocks.get(&edited) == Some(&BlockKind::Crystal)
        );
        assert_eq!(
            app.world_mut()
                .query::<&WorldChunk>()
                .iter(app.world())
                .count(),
            1
        );
    }
    #[test]
    fn negative_chunk_coordinates_use_floor_division() {
        assert_eq!(chunk_of(IVec3::new(-1, 0, -17)), IVec2::new(-1, -2));
    }
    #[test]
    fn chunk_generation_matches_whole_region_and_reloads_edits() {
        let generator = terrain::Generator {
            seed: terrain::DEFAULT_SEED,
        };
        let mut whole = WorldData::default();
        generator.generate_region(&mut whole, IVec2::splat(-16), IVec2::splat(15));
        let mut chunk = WorldData::default();
        generator.generate_region(&mut chunk, IVec2::splat(-16), IVec2::splat(-1));
        for (&p, &kind) in &chunk.blocks {
            assert!(whole.blocks.get(&p) == Some(&kind));
        }
        assert_eq!(
            chunk.blocks.len(),
            whole
                .blocks
                .keys()
                .filter(|p| chunk_of(**p) == IVec2::splat(-1))
                .count()
        );
        let removed = IVec3::new(-4, WORLD_BOTTOM + 1, -4);
        let placed = IVec3::new(-4, 60, -4);
        let edits = HashMap::from([(removed, None), (placed, Some(BlockKind::Crystal))]);
        apply_edits(&mut chunk, &edits, IVec2::splat(-1));
        generator.generate_region(&mut chunk, IVec2::splat(-16), IVec2::splat(-1));
        apply_edits(&mut chunk, &edits, IVec2::splat(-1));
        assert!(!chunk.blocks.contains_key(&removed));
        assert!(chunk.blocks.get(&placed) == Some(&BlockKind::Crystal));
        assert_eq!(chunk.max_y, 60);
    }
}
