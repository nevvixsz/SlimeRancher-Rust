//! Test arena ("пробная карта") setup with floor, perimeter walls, corral, pond, garden,
//! plort market dropzone, and initial actors for early physics & vacpack testing.

use sr_ai::{GordoRewardKind, WorldGordo};
use sr_gameplay::PlotBounds;
use sr_ids::{IdentifiableId, LandPlotId, RegionSetId};
use sr_math::Vec3;
use sr_model::LandPlotModel;
use sr_physics::{CollisionLayers, StaticCollider, StaticShape};
use sr_sim::Simulation;

/// Coordinates of key landmarks inside the flat 100x100 test arena.
pub mod landmarks {
    use sr_math::Vec3;

    pub const PLAYER_SPAWN: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const CORRAL_CENTER: Vec3 = Vec3::new(14.0, 0.0, 14.0);
    pub const GARDEN_CENTER: Vec3 = Vec3::new(-14.0, 0.0, 14.0);
    pub const POND_CENTER: Vec3 = Vec3::new(-14.0, 0.0, -14.0);
    pub const MARKET_CENTER: Vec3 = Vec3::new(14.0, 0.0, -14.0);
}

/// Populates `sim` with static colliders, land plots, and starter actors for testing.
pub fn setup_test_arena(sim: &mut Simulation) {
    sim.physics.statics.clear();
    sim.model.player.position = landmarks::PLAYER_SPAWN;
    sim.player_ctrl.wake_up_pos = landmarks::PLAYER_SPAWN;

    // 1. Main arena floor (y = 0) & sea kill plane below arena (y = -8)
    sim.physics.add_static(StaticCollider {
        id: "arena_floor".into(),
        layers: CollisionLayers::WORLD,
        is_trigger: false,
        shape: StaticShape::Aabb {
            center: Vec3::new(0.0, -0.5, 0.0),
            half_extents: Vec3::new(50.0, 0.5, 50.0),
        },
    });
    sim.physics.add_static(StaticCollider {
        id: "sea_kill_plane".into(),
        layers: CollisionLayers::SEA_KILL,
        is_trigger: true,
        shape: StaticShape::GroundPlane { height: -8.0 },
    });

    // 2. Outer perimeter low walls (x = +/-50, z = +/-50, except one cliff edge for sea-fall testing at z < -50)
    for (id, center, half) in [
        ("wall_north", Vec3::new(0.0, 2.0, 50.0), Vec3::new(50.0, 2.0, 0.5)),
        ("wall_east", Vec3::new(50.0, 2.0, 0.0), Vec3::new(0.5, 2.0, 50.0)),
        ("wall_west", Vec3::new(-50.0, 2.0, 0.0), Vec3::new(0.5, 2.0, 50.0)),
    ] {
        sim.physics.add_static(StaticCollider {
            id: id.into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::Aabb {
                center,
                half_extents: half,
            },
        });
    }

    // 3. Test Corral (4 walls around (14, 0, 14), 10x10 interior)
    let cc = landmarks::CORRAL_CENTER;
    for (id, offset, half) in [
        ("corral_n", Vec3::new(0.0, 1.5, 5.25), Vec3::new(5.5, 1.5, 0.25)),
        ("corral_s", Vec3::new(0.0, 1.5, -5.25), Vec3::new(5.5, 1.5, 0.25)),
        ("corral_e", Vec3::new(5.25, 1.5, 0.0), Vec3::new(0.25, 1.5, 5.5)),
        ("corral_w", Vec3::new(-5.25, 1.5, 0.0), Vec3::new(0.25, 1.5, 5.5)),
    ] {
        sim.physics.add_static(StaticCollider {
            id: id.into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::Aabb {
                center: cc + offset,
                half_extents: half,
            },
        });
    }
    sim.model.land_plots.insert(
        "sandbox_corral".into(),
        LandPlotModel::new("sandbox_corral", LandPlotId::Corral),
    );
    sim.model.land_plots.insert(
        "sandbox_garden".into(),
        LandPlotModel::new("sandbox_garden", LandPlotId::Garden),
    );
    sim.model.land_plots.insert(
        "sandbox_pond".into(),
        LandPlotModel::new("sandbox_pond", LandPlotId::Pond),
    );
    sim.ranch.register_bounds(PlotBounds {
        plot_id: "sandbox_corral".into(),
        center: landmarks::CORRAL_CENTER,
        half_extents: Vec3::new(5.0, 3.0, 5.0),
        region: RegionSetId::Home,
    });
    sim.ranch.register_bounds(PlotBounds {
        plot_id: "sandbox_garden".into(),
        center: landmarks::GARDEN_CENTER,
        half_extents: Vec3::new(5.0, 3.0, 5.0),
        region: RegionSetId::Home,
    });
    sim.ranch.register_bounds(PlotBounds {
        plot_id: "sandbox_pond".into(),
        center: landmarks::POND_CENTER,
        half_extents: Vec3::new(4.0, 2.0, 4.0),
        region: RegionSetId::Home,
    });

    // 4. Water Pond trigger (`LiquidSource` around (-14, 0, -14))
    sim.physics.add_static(StaticCollider {
        id: "sandbox_pond".into(),
        layers: CollisionLayers::LIQUID,
        is_trigger: true,
        shape: StaticShape::Aabb {
            center: landmarks::POND_CENTER + Vec3::Y * 0.25,
            half_extents: Vec3::new(4.0, 0.5, 4.0),
        },
    });

    // 5. Plort Market terminal box at (14, 0, -14)
    sim.physics.add_static(StaticCollider {
        id: "sandbox_market".into(),
        layers: CollisionLayers::WORLD,
        is_trigger: false,
        shape: StaticShape::Aabb {
            center: landmarks::MARKET_CENTER + Vec3::Y * 1.25,
            half_extents: Vec3::new(2.0, 1.25, 1.0),
        },
    });

    // 6. Starter actors for immediate vacpack / physics testing
    let starters = [
        (IdentifiableId::PinkSlime, Vec3::new(0.0, 1.0, 4.0)),
        (IdentifiableId::PinkSlime, Vec3::new(1.5, 1.0, 5.0)),
        (IdentifiableId::TabbySlime, Vec3::new(-1.5, 1.0, 5.5)),
        (IdentifiableId::RockSlime, landmarks::CORRAL_CENTER + Vec3::new(0.0, 1.0, 0.0)),
        (IdentifiableId::CarrotVeggie, landmarks::GARDEN_CENTER + Vec3::new(0.0, 0.5, 0.0)),
        (IdentifiableId::CuberryFruit, landmarks::GARDEN_CENTER + Vec3::new(1.2, 0.5, 0.5)),
        (IdentifiableId::CrateReef01, Vec3::new(4.0, 1.0, 3.0)),
    ];
    for (ident, pos) in starters {
        sim.spawn_actor(ident, RegionSetId::Home, pos, Vec3::ZERO);
    }

    // 7. Arena Pink Gordo at (0, 0, -14) (30 food to pop -> SlimeKey + 3 Pink Slimes)
    let gordo_pos = Vec3::new(0.0, 0.0, -14.0);
    if !sim.model.gordos.contains_key("gordo_arena_pink") {
        sim.ai.register_gordo(
            &mut sim.model,
            WorldGordo {
                gordo_id: "gordo_arena_pink".into(),
                slime_ident: IdentifiableId::PinkSlime,
                pos: gordo_pos,
                region: RegionSetId::Home,
                rewards: vec![
                    GordoRewardKind::SlimeKey,
                    GordoRewardKind::CratesAndSlimes {
                        crate_id: IdentifiableId::CrateReef01,
                        crate_count: 2,
                    },
                ],
                spawn_slime_count: 3,
            },
            30,
        );
    }
}

/// Renders an ASCII top-down radar map (`[-24, 24]` around origin) for CLI inspection.
pub fn render_ascii_map(sim: &Simulation) -> String {
    const W: usize = 33;
    const H: usize = 17;
    let mut grid = vec![vec!['.'; W]; H];

    let to_cell = |pos: Vec3| -> Option<(usize, usize)> {
        let gx = ((pos.x + 24.0) / 48.0 * (W - 1) as f32).round() as isize;
        let gz = ((24.0 - pos.z) / 48.0 * (H - 1) as f32).round() as isize;
        if gx >= 0 && gx < W as isize && gz >= 0 && gz < H as isize {
            Some((gx as usize, gz as usize))
        } else {
            None
        }
    };

    if let Some((x, z)) = to_cell(landmarks::CORRAL_CENTER) {
        grid[z][x] = 'C';
    }
    if let Some((x, z)) = to_cell(landmarks::GARDEN_CENTER) {
        grid[z][x] = 'G';
    }
    if let Some((x, z)) = to_cell(landmarks::POND_CENTER) {
        grid[z][x] = '~';
    }
    if let Some((x, z)) = to_cell(landmarks::MARKET_CENTER) {
        grid[z][x] = '$';
    }
    for actor in sim.model.actors.values() {
        if let Some((x, z)) = to_cell(actor.pos) {
            grid[z][x] = if actor.ident.is_slime() {
                'S'
            } else if actor.ident.is_plort() {
                'p'
            } else if actor.ident.is_food() {
                'f'
            } else if actor.ident.is_standard_crate() {
                '#'
            } else {
                '*'
            };
        }
    }
    for g in &sim.ai.world_gordos {
        let popped = sim
            .model
            .gordos
            .get(&g.gordo_id)
            .map_or(false, |gm| gm.gordo_eaten_count == -1);
        if !popped {
            if let Some((x, z)) = to_cell(g.pos) {
                grid[z][x] = 'O';
            }
        }
    }
    if let Some((x, z)) = to_cell(sim.model.player.position) {
        grid[z][x] = '@';
    }

    let mut out = String::from("Legend: @=Player S=Slime O=Gordo p=Plort f=Food #=Crate C=Corral G=Garden ~=Pond $=Market\n");
    for row in grid {
        let line: String = row.into_iter().collect();
        out.push_str(&line);
        out.push('\n');
    }
    out
}
