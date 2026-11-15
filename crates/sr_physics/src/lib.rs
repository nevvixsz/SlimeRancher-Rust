//! 3D physics world, raycast batching, and `vp_FPController` + `EnergyJetpack` player motor.

pub mod player_motor;
pub mod shapes;
pub mod world;

pub use player_motor::{
    PlayerMotor, PlayerMoveInput, JETPACK_ENERGY_PER_SECOND, JETPACK_HOVER_HEIGHT,
    JETPACK_INIT_ENERGY_USED, JUMP_VELOCITY, SPRINT_SPEED, WALK_SPEED,
};
pub use shapes::{
    intersect_ray_aabb, intersect_ray_sphere, CollisionLayers, RayHit, StaticCollider, StaticShape,
};
pub use world::{ImpactEvent, PhysicsWorld, RayQuery, RigidBody, TriggerOverlap, DEFAULT_GRAVITY};

#[cfg(test)]
mod tests {
    use super::*;
    use sr_math::Vec3;
    use sr_model::PlayerModel;

    #[test]
    fn rigidbody_falls_bounces_and_triggers_impact() {
        let mut pw = PhysicsWorld::new();
        pw.add_static(StaticCollider {
            id: "floor".into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::GroundPlane { height: 0.0 },
        });
        pw.insert_body(RigidBody {
            actor_id: 100,
            pos: Vec3::new(0.0, 10.0, 0.0),
            vel: Vec3::new(0.0, -16.0, 0.0),
            radius: 0.5,
            ..Default::default()
        });

        let mut max_impact = 0.0f32;
        for _ in 0..60 {
            let (impacts, _) = pw.step(1.0 / 60.0);
            for imp in impacts {
                max_impact = max_impact.max(imp.normal_speed);
            }
        }
        assert!(max_impact > 14.0);
        assert!(pw.bodies[&100].pos.y >= 0.49);
    }

    #[test]
    fn player_jump_sprint_and_jetpack_hover() {
        let mut pw = PhysicsWorld::new();
        pw.add_static(StaticCollider {
            id: "floor".into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::GroundPlane { height: 0.0 },
        });
        let mut player = PlayerModel::default();
        player.has_jetpack = true;
        let mut motor = PlayerMotor::default();

        // Jump at t = 32400.0
        motor.step(
            &mut player,
            &pw,
            PlayerMoveInput {
                jump_held: true,
                jump_pressed: true,
                ..Default::default()
            },
            1.0 / 60.0,
            32400.0,
        );
        assert!(!motor.grounded);
        assert!(motor.velocity.y > 7.0);

        // Advance past jump-to-jetpack delay (18 world seconds) and let velocity drop <= 0.5
        let mut t = 32401.0;
        let mut jetpack_engaged = false;
        for _ in 0..60 {
            t += 1.0; // world time advances 60x in game, use >18s after jump
            motor.step(
                &mut player,
                &pw,
                PlayerMoveInput {
                    jump_held: true,
                    ..Default::default()
                },
                1.0 / 60.0,
                t + 20.0,
            );
            if motor.jetpack_active {
                jetpack_engaged = true;
                break;
            }
        }
        assert!(jetpack_engaged);
        assert!(player.curr_energy < 100.0);
    }

    #[test]
    fn raycast_batch_and_cone_overlap_work() {
        let mut pw = PhysicsWorld::new();
        pw.add_static(StaticCollider {
            id: "wall".into(),
            layers: CollisionLayers::WORLD,
            is_trigger: false,
            shape: StaticShape::Aabb {
                center: Vec3::new(0.0, 1.0, 10.0),
                half_extents: Vec3::new(5.0, 2.0, 0.5),
            },
        });
        pw.insert_body(RigidBody {
            actor_id: 100,
            pos: Vec3::new(0.0, 1.0, 5.0),
            radius: 0.5,
            ..Default::default()
        });
        let hits = pw.raycast_batch(&[RayQuery {
            origin: Vec3::new(0.0, 1.0, 0.0),
            dir: Vec3::Z,
            max_dist: 20.0,
            mask: CollisionLayers::ALL_SOLID,
            include_triggers: false,
        }]);
        assert_eq!(hits[0].as_ref().unwrap().actor_id, Some(100));

        let in_cone = pw.overlap_cone(Vec3::new(0.0, 1.0, 0.0), Vec3::Z, 10.0, 0.85);
        assert_eq!(in_cone, vec![100]);
    }
}
