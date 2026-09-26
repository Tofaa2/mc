use super::*;

#[test]
#[ignore = "manual deterministic CPU meshing benchmark"]
fn benchmark_chunk_meshing() {
    let mut world = WorldData::default();
    terrain::Generator { seed: 71024 }.generate_region(&mut world, IVec2::ZERO, IVec2::splat(47));
    let start = std::time::Instant::now();
    let mut vertices = 0;
    for _ in 0..20 {
        for x in 0..3 {
            for z in 0..3 {
                let mesh = std::hint::black_box(build_chunk_mesh(&world, IVec2::new(x, z)));
                vertices += mesh.count_vertices();
            }
        }
    }
    eprintln!(
        "180 meshes: {:.2} ms; vertices: {vertices}",
        start.elapsed().as_secs_f64() * 1000.
    );
}

#[test]
fn chunk_faces_point_outward_and_cull_shared_boundaries() {
    use bevy::render::mesh::VertexAttributeValues;
    let mut world = WorldData::default();
    world.blocks.insert(IVec3::new(15, 0, 0), BlockKind::Grass);
    let mesh = build_chunk_mesh(&world, IVec2::ZERO);
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions")
    };
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!("normals")
    };
    assert_eq!(positions.len(), 24);
    for face in 0..6 {
        let i = face * 4;
        let a = Vec3::from_array(positions[i]);
        let b = Vec3::from_array(positions[i + 1]);
        let c = Vec3::from_array(positions[i + 2]);
        assert!((b - a).cross(c - a).dot(Vec3::from_array(normals[i])) > 0.0);
    }
    world.blocks.insert(IVec3::new(16, 0, 0), BlockKind::Dirt);
    assert_eq!(build_chunk_mesh(&world, IVec2::ZERO).count_vertices(), 20);
    assert_eq!(build_chunk_mesh(&world, IVec2::X).count_vertices(), 20);
    assert_eq!(build_chunk_mesh(&world, -IVec2::X).count_vertices(), 0);
}

#[test]
fn leaf_geometry_is_finite_and_has_real_surface_area() {
    use bevy::render::mesh::VertexAttributeValues;
    let mesh = leaf_cluster_mesh();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions")
    };
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!("normals")
    };
    assert!(positions.len() > 24, "Canopy must contain multiple blocks");
    for (index, face) in positions.as_chunks::<4>().0.iter().enumerate() {
        let [a, b, c] = [face[0], face[1], face[2]].map(Vec3::from_array);
        assert!(a.is_finite() && b.is_finite() && c.is_finite());
        assert!((b - a).cross(c - a).length_squared() > 0.000001);
        let normal = Vec3::from_array(normals[index * 4]);
        assert_eq!(
            normal.abs().element_sum(),
            1.0,
            "Voxel faces must be axis aligned"
        );
        assert!(
            (b - a).cross(c - a).dot(normal) > 0.0,
            "Canopy faces must point outward"
        );
    }
}

#[test]
fn canopy_has_open_faces_and_recessed_leaf_layers() {
    use bevy::render::mesh::VertexAttributeValues;
    let mesh = leaf_cluster_mesh();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions")
    };
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!("normals")
    };
    let mut outside = 0;
    let mut inside = 0;
    for (i, face) in positions.as_chunks::<4>().0.iter().enumerate() {
        let center = face.iter().map(|p| Vec3::from_array(*p)).sum::<Vec3>() / 4.0;
        if normals[i * 4] != [1., 0., 0.]
            || !(0.0..1.0).contains(&center.y)
            || !(0.0..1.0).contains(&center.z)
        {
            continue;
        }
        if (center.x - 3.0).abs() < 0.001 {
            outside += 1;
        }
        if (center.x - 2.72).abs() < 0.001 {
            inside += 1;
        }
    }
    assert!(
        outside > 0 && outside < 36,
        "Outer face must have leaves AND holes"
    );
    assert!(
        inside > 0 && inside < 36,
        "Inset layer must also be perforated"
    );
    assert!(inside < outside, "Interior foliage should be sparser");
}

#[test]
fn generated_trees_have_intact_trunks_and_rendered_canopies() {
    let mut world = WorldData::default();
    generate_world(&mut world);
    let caps: Vec<_> = world
        .blocks
        .iter()
        .filter_map(|(pos, kind)| (*kind == BlockKind::Planks).then_some(*pos))
        .collect();
    assert!(caps.len() > 10, "Expected a grove of canopy anchors");
    for cap in &caps {
        for offset in 1..=3 {
            assert!(world.blocks.get(&(*cap - IVec3::Y * offset)) == Some(&BlockKind::Wood));
        }
    }
    let mut app = App::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    {
        let mut commands = Commands::new(&mut queue, app.world());
        let foliage = create_foliage_assets(&mut meshes, &mut materials);
        spawn_foliage(&mut commands, &foliage, &world);
    }
    queue.apply(app.world_mut());
    let count = app.world_mut().query::<&Sway>().iter(app.world()).count();
    assert_eq!(count, caps.len(), "Every tree must spawn its canopy");
}
