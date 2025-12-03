//! Large-scale archetype benchmark simulating production ECS workloads
//!
//! This benchmark creates a realistic scenario with:
//! - 500 archetypes, each with 8 component types (max supported by Bundle)
//! - 400 archetypes containing Position
//! - 200 archetypes containing Velocity
//! - 1000 entities per archetype (500,000 total entities)
//!
//! This tests how well the ECS scales with many archetypes, which is common
//! in production games where entities have diverse component combinations.
//!
//! Run with: cargo bench --bench archetype_scale

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ============================================================================
// Generate filler components to create unique archetypes
// ============================================================================

macro_rules! define_filler_components {
    ($mod_name:ident, $derive:path, $($name:ident),*) => {
        #[allow(dead_code)]
        mod $mod_name {
            $(
                #[derive($derive, Clone, Copy, Default)]
                pub struct $name(#[allow(dead_code)] pub u32);
            )*
        }
    };
}

// ============================================================================
// Nimbus ECS types and setup
// ============================================================================

mod nimbus {
    use nimbus_ecs::{App, Component, Query, World};

    #[derive(Component, Clone, Copy, Default)]
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy, Default)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    // Filler components to create unique archetypes
    // We need enough to create 500 unique archetypes
    define_filler_components!(
        filler, nimbus_ecs::Component,
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9,
        C10, C11, C12, C13, C14, C15, C16, C17, C18, C19,
        C20, C21, C22, C23, C24, C25, C26, C27, C28, C29,
        C30, C31, C32, C33, C34, C35, C36, C37, C38, C39
    );

    /// Spawn entities for archetype pattern based on index
    /// Returns world with 500 archetypes × 1000 entities = 500,000 entities
    pub fn setup_world() -> World {
        let mut world = World::new();

        // Create 500 archetypes with 1000 entities each
        // Distribution:
        // - Archetypes 0-199: Position + Velocity + 6 fillers (200 archetypes)
        // - Archetypes 200-399: Position only + 7 fillers (200 archetypes)
        // - Archetypes 400-499: Neither + 8 fillers (100 archetypes)

        for arch_idx in 0u32..500 {
            let has_position = arch_idx < 400;
            let has_velocity = arch_idx < 200;

            // Generate a unique filler set based on arch_idx
            // We use different combinations to ensure unique archetypes
            let filler_base = (arch_idx % 40) as usize;

            for entity_idx in 0..1000 {
                let f = entity_idx as f32;

                // Use macro to avoid repetition - each branch spawns with 8 components
                match (has_position, has_velocity) {
                    (true, true) => {
                        // Position + Velocity + 6 fillers = 8 components
                        match filler_base % 20 {
                            0 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0))); }
                            1 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C6(arch_idx), filler::C7(0), filler::C8(0), filler::C9(0), filler::C10(0), filler::C11(0))); }
                            2 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C12(arch_idx), filler::C13(0), filler::C14(0), filler::C15(0), filler::C16(0), filler::C17(0))); }
                            3 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C18(arch_idx), filler::C19(0), filler::C20(0), filler::C21(0), filler::C22(0), filler::C23(0))); }
                            4 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C24(arch_idx), filler::C25(0), filler::C26(0), filler::C27(0), filler::C28(0), filler::C29(0))); }
                            5 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C30(arch_idx), filler::C31(0), filler::C32(0), filler::C33(0), filler::C34(0), filler::C35(0))); }
                            6 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0))); }
                            7 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0))); }
                            8 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C12(arch_idx), filler::C14(0), filler::C16(0), filler::C18(0), filler::C20(0), filler::C22(0))); }
                            9 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C13(arch_idx), filler::C15(0), filler::C17(0), filler::C19(0), filler::C21(0), filler::C23(0))); }
                            10 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C24(arch_idx), filler::C26(0), filler::C28(0), filler::C30(0), filler::C32(0), filler::C34(0))); }
                            11 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C25(arch_idx), filler::C27(0), filler::C29(0), filler::C31(0), filler::C33(0), filler::C35(0))); }
                            12 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C5(0), filler::C10(0), filler::C15(0), filler::C20(0), filler::C25(0))); }
                            13 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C1(arch_idx), filler::C6(0), filler::C11(0), filler::C16(0), filler::C21(0), filler::C26(0))); }
                            14 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C2(arch_idx), filler::C7(0), filler::C12(0), filler::C17(0), filler::C22(0), filler::C27(0))); }
                            15 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C3(arch_idx), filler::C8(0), filler::C13(0), filler::C18(0), filler::C23(0), filler::C28(0))); }
                            16 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C4(arch_idx), filler::C9(0), filler::C14(0), filler::C19(0), filler::C24(0), filler::C29(0))); }
                            17 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C0(0), filler::C1(0))); }
                            18 => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C2(0), filler::C3(0))); }
                            _ => { world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C4(0), filler::C5(0))); }
                        }
                    }
                    (true, false) => {
                        // Position + 7 fillers = 8 components
                        match filler_base % 20 {
                            0 => { world.spawn_with((Position { x: f, y: f }, filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0), filler::C6(0))); }
                            1 => { world.spawn_with((Position { x: f, y: f }, filler::C7(arch_idx), filler::C8(0), filler::C9(0), filler::C10(0), filler::C11(0), filler::C12(0), filler::C13(0))); }
                            2 => { world.spawn_with((Position { x: f, y: f }, filler::C14(arch_idx), filler::C15(0), filler::C16(0), filler::C17(0), filler::C18(0), filler::C19(0), filler::C20(0))); }
                            3 => { world.spawn_with((Position { x: f, y: f }, filler::C21(arch_idx), filler::C22(0), filler::C23(0), filler::C24(0), filler::C25(0), filler::C26(0), filler::C27(0))); }
                            4 => { world.spawn_with((Position { x: f, y: f }, filler::C28(arch_idx), filler::C29(0), filler::C30(0), filler::C31(0), filler::C32(0), filler::C33(0), filler::C34(0))); }
                            5 => { world.spawn_with((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C0(0), filler::C1(0))); }
                            6 => { world.spawn_with((Position { x: f, y: f }, filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0), filler::C12(0))); }
                            7 => { world.spawn_with((Position { x: f, y: f }, filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0), filler::C13(0))); }
                            8 => { world.spawn_with((Position { x: f, y: f }, filler::C14(arch_idx), filler::C16(0), filler::C18(0), filler::C20(0), filler::C22(0), filler::C24(0), filler::C26(0))); }
                            9 => { world.spawn_with((Position { x: f, y: f }, filler::C15(arch_idx), filler::C17(0), filler::C19(0), filler::C21(0), filler::C23(0), filler::C25(0), filler::C27(0))); }
                            10 => { world.spawn_with((Position { x: f, y: f }, filler::C28(arch_idx), filler::C30(0), filler::C32(0), filler::C34(0), filler::C36(0), filler::C38(0), filler::C0(0))); }
                            11 => { world.spawn_with((Position { x: f, y: f }, filler::C29(arch_idx), filler::C31(0), filler::C33(0), filler::C35(0), filler::C37(0), filler::C39(0), filler::C1(0))); }
                            12 => { world.spawn_with((Position { x: f, y: f }, filler::C0(arch_idx), filler::C5(0), filler::C10(0), filler::C15(0), filler::C20(0), filler::C25(0), filler::C30(0))); }
                            13 => { world.spawn_with((Position { x: f, y: f }, filler::C1(arch_idx), filler::C6(0), filler::C11(0), filler::C16(0), filler::C21(0), filler::C26(0), filler::C31(0))); }
                            14 => { world.spawn_with((Position { x: f, y: f }, filler::C2(arch_idx), filler::C7(0), filler::C12(0), filler::C17(0), filler::C22(0), filler::C27(0), filler::C32(0))); }
                            15 => { world.spawn_with((Position { x: f, y: f }, filler::C3(arch_idx), filler::C8(0), filler::C13(0), filler::C18(0), filler::C23(0), filler::C28(0), filler::C33(0))); }
                            16 => { world.spawn_with((Position { x: f, y: f }, filler::C4(arch_idx), filler::C9(0), filler::C14(0), filler::C19(0), filler::C24(0), filler::C29(0), filler::C34(0))); }
                            17 => { world.spawn_with((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C2(0), filler::C3(0))); }
                            18 => { world.spawn_with((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C4(0), filler::C5(0))); }
                            _ => { world.spawn_with((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C6(0), filler::C7(0))); }
                        }
                    }
                    (false, _) => {
                        // 8 fillers only
                        match filler_base % 10 {
                            0 => { world.spawn_with((filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0), filler::C6(0), filler::C7(0))); }
                            1 => { world.spawn_with((filler::C8(arch_idx), filler::C9(0), filler::C10(0), filler::C11(0), filler::C12(0), filler::C13(0), filler::C14(0), filler::C15(0))); }
                            2 => { world.spawn_with((filler::C16(arch_idx), filler::C17(0), filler::C18(0), filler::C19(0), filler::C20(0), filler::C21(0), filler::C22(0), filler::C23(0))); }
                            3 => { world.spawn_with((filler::C24(arch_idx), filler::C25(0), filler::C26(0), filler::C27(0), filler::C28(0), filler::C29(0), filler::C30(0), filler::C31(0))); }
                            4 => { world.spawn_with((filler::C32(arch_idx), filler::C33(0), filler::C34(0), filler::C35(0), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0))); }
                            5 => { world.spawn_with((filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0), filler::C12(0), filler::C14(0))); }
                            6 => { world.spawn_with((filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0), filler::C13(0), filler::C15(0))); }
                            7 => { world.spawn_with((filler::C16(arch_idx), filler::C18(0), filler::C20(0), filler::C22(0), filler::C24(0), filler::C26(0), filler::C28(0), filler::C30(0))); }
                            8 => { world.spawn_with((filler::C17(arch_idx), filler::C19(0), filler::C21(0), filler::C23(0), filler::C25(0), filler::C27(0), filler::C29(0), filler::C31(0))); }
                            _ => { world.spawn_with((filler::C32(arch_idx), filler::C34(0), filler::C36(0), filler::C38(0), filler::C33(0), filler::C35(0), filler::C37(0), filler::C39(0))); }
                        }
                    }
                }
            }
        }

        world
    }

    pub fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
        for (pos, vel) in query.iter() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    pub fn position_only_system(mut query: Query<&mut Position>) {
        for pos in query.iter() {
            pos.x += 1.0;
            pos.y += 1.0;
        }
    }
    
    /// Wrap setup_world in an App for scheduler benchmarks
    pub fn setup_app() -> App {
        App::from_parts(setup_world(), nimbus_ecs::scheduler::PriorityScheduler::new())
    }
}

// ============================================================================
// Bevy ECS types and setup
// ============================================================================

mod bevy {
    use bevy_ecs::prelude::*;

    #[derive(Component, Clone, Copy, Default)]
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy, Default)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    // Filler components to create unique archetypes
    define_filler_components!(
        filler, bevy_ecs::prelude::Component,
        C0, C1, C2, C3, C4, C5, C6, C7, C8, C9,
        C10, C11, C12, C13, C14, C15, C16, C17, C18, C19,
        C20, C21, C22, C23, C24, C25, C26, C27, C28, C29,
        C30, C31, C32, C33, C34, C35, C36, C37, C38, C39
    );

    /// Setup world with same distribution as nimbus
    pub fn setup_world() -> World {
        let mut world = World::new();

        for arch_idx in 0u32..500 {
            let has_position = arch_idx < 400;
            let has_velocity = arch_idx < 200;
            let filler_base = (arch_idx % 40) as usize;

            for entity_idx in 0..1000 {
                let f = entity_idx as f32;

                match (has_position, has_velocity) {
                    (true, true) => {
                        match filler_base % 20 {
                            0 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0))); }
                            1 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C6(arch_idx), filler::C7(0), filler::C8(0), filler::C9(0), filler::C10(0), filler::C11(0))); }
                            2 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C12(arch_idx), filler::C13(0), filler::C14(0), filler::C15(0), filler::C16(0), filler::C17(0))); }
                            3 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C18(arch_idx), filler::C19(0), filler::C20(0), filler::C21(0), filler::C22(0), filler::C23(0))); }
                            4 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C24(arch_idx), filler::C25(0), filler::C26(0), filler::C27(0), filler::C28(0), filler::C29(0))); }
                            5 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C30(arch_idx), filler::C31(0), filler::C32(0), filler::C33(0), filler::C34(0), filler::C35(0))); }
                            6 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0))); }
                            7 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0))); }
                            8 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C12(arch_idx), filler::C14(0), filler::C16(0), filler::C18(0), filler::C20(0), filler::C22(0))); }
                            9 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C13(arch_idx), filler::C15(0), filler::C17(0), filler::C19(0), filler::C21(0), filler::C23(0))); }
                            10 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C24(arch_idx), filler::C26(0), filler::C28(0), filler::C30(0), filler::C32(0), filler::C34(0))); }
                            11 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C25(arch_idx), filler::C27(0), filler::C29(0), filler::C31(0), filler::C33(0), filler::C35(0))); }
                            12 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C0(arch_idx), filler::C5(0), filler::C10(0), filler::C15(0), filler::C20(0), filler::C25(0))); }
                            13 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C1(arch_idx), filler::C6(0), filler::C11(0), filler::C16(0), filler::C21(0), filler::C26(0))); }
                            14 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C2(arch_idx), filler::C7(0), filler::C12(0), filler::C17(0), filler::C22(0), filler::C27(0))); }
                            15 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C3(arch_idx), filler::C8(0), filler::C13(0), filler::C18(0), filler::C23(0), filler::C28(0))); }
                            16 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C4(arch_idx), filler::C9(0), filler::C14(0), filler::C19(0), filler::C24(0), filler::C29(0))); }
                            17 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C0(0), filler::C1(0))); }
                            18 => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C2(0), filler::C3(0))); }
                            _ => { world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }, filler::C36(arch_idx), filler::C37(0), filler::C38(0), filler::C39(0), filler::C4(0), filler::C5(0))); }
                        }
                    }
                    (true, false) => {
                        match filler_base % 20 {
                            0 => { world.spawn((Position { x: f, y: f }, filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0), filler::C6(0))); }
                            1 => { world.spawn((Position { x: f, y: f }, filler::C7(arch_idx), filler::C8(0), filler::C9(0), filler::C10(0), filler::C11(0), filler::C12(0), filler::C13(0))); }
                            2 => { world.spawn((Position { x: f, y: f }, filler::C14(arch_idx), filler::C15(0), filler::C16(0), filler::C17(0), filler::C18(0), filler::C19(0), filler::C20(0))); }
                            3 => { world.spawn((Position { x: f, y: f }, filler::C21(arch_idx), filler::C22(0), filler::C23(0), filler::C24(0), filler::C25(0), filler::C26(0), filler::C27(0))); }
                            4 => { world.spawn((Position { x: f, y: f }, filler::C28(arch_idx), filler::C29(0), filler::C30(0), filler::C31(0), filler::C32(0), filler::C33(0), filler::C34(0))); }
                            5 => { world.spawn((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C0(0), filler::C1(0))); }
                            6 => { world.spawn((Position { x: f, y: f }, filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0), filler::C12(0))); }
                            7 => { world.spawn((Position { x: f, y: f }, filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0), filler::C13(0))); }
                            8 => { world.spawn((Position { x: f, y: f }, filler::C14(arch_idx), filler::C16(0), filler::C18(0), filler::C20(0), filler::C22(0), filler::C24(0), filler::C26(0))); }
                            9 => { world.spawn((Position { x: f, y: f }, filler::C15(arch_idx), filler::C17(0), filler::C19(0), filler::C21(0), filler::C23(0), filler::C25(0), filler::C27(0))); }
                            10 => { world.spawn((Position { x: f, y: f }, filler::C28(arch_idx), filler::C30(0), filler::C32(0), filler::C34(0), filler::C36(0), filler::C38(0), filler::C0(0))); }
                            11 => { world.spawn((Position { x: f, y: f }, filler::C29(arch_idx), filler::C31(0), filler::C33(0), filler::C35(0), filler::C37(0), filler::C39(0), filler::C1(0))); }
                            12 => { world.spawn((Position { x: f, y: f }, filler::C0(arch_idx), filler::C5(0), filler::C10(0), filler::C15(0), filler::C20(0), filler::C25(0), filler::C30(0))); }
                            13 => { world.spawn((Position { x: f, y: f }, filler::C1(arch_idx), filler::C6(0), filler::C11(0), filler::C16(0), filler::C21(0), filler::C26(0), filler::C31(0))); }
                            14 => { world.spawn((Position { x: f, y: f }, filler::C2(arch_idx), filler::C7(0), filler::C12(0), filler::C17(0), filler::C22(0), filler::C27(0), filler::C32(0))); }
                            15 => { world.spawn((Position { x: f, y: f }, filler::C3(arch_idx), filler::C8(0), filler::C13(0), filler::C18(0), filler::C23(0), filler::C28(0), filler::C33(0))); }
                            16 => { world.spawn((Position { x: f, y: f }, filler::C4(arch_idx), filler::C9(0), filler::C14(0), filler::C19(0), filler::C24(0), filler::C29(0), filler::C34(0))); }
                            17 => { world.spawn((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C2(0), filler::C3(0))); }
                            18 => { world.spawn((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C4(0), filler::C5(0))); }
                            _ => { world.spawn((Position { x: f, y: f }, filler::C35(arch_idx), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0), filler::C6(0), filler::C7(0))); }
                        }
                    }
                    (false, _) => {
                        match filler_base % 10 {
                            0 => { world.spawn((filler::C0(arch_idx), filler::C1(0), filler::C2(0), filler::C3(0), filler::C4(0), filler::C5(0), filler::C6(0), filler::C7(0))); }
                            1 => { world.spawn((filler::C8(arch_idx), filler::C9(0), filler::C10(0), filler::C11(0), filler::C12(0), filler::C13(0), filler::C14(0), filler::C15(0))); }
                            2 => { world.spawn((filler::C16(arch_idx), filler::C17(0), filler::C18(0), filler::C19(0), filler::C20(0), filler::C21(0), filler::C22(0), filler::C23(0))); }
                            3 => { world.spawn((filler::C24(arch_idx), filler::C25(0), filler::C26(0), filler::C27(0), filler::C28(0), filler::C29(0), filler::C30(0), filler::C31(0))); }
                            4 => { world.spawn((filler::C32(arch_idx), filler::C33(0), filler::C34(0), filler::C35(0), filler::C36(0), filler::C37(0), filler::C38(0), filler::C39(0))); }
                            5 => { world.spawn((filler::C0(arch_idx), filler::C2(0), filler::C4(0), filler::C6(0), filler::C8(0), filler::C10(0), filler::C12(0), filler::C14(0))); }
                            6 => { world.spawn((filler::C1(arch_idx), filler::C3(0), filler::C5(0), filler::C7(0), filler::C9(0), filler::C11(0), filler::C13(0), filler::C15(0))); }
                            7 => { world.spawn((filler::C16(arch_idx), filler::C18(0), filler::C20(0), filler::C22(0), filler::C24(0), filler::C26(0), filler::C28(0), filler::C30(0))); }
                            8 => { world.spawn((filler::C17(arch_idx), filler::C19(0), filler::C21(0), filler::C23(0), filler::C25(0), filler::C27(0), filler::C29(0), filler::C31(0))); }
                            _ => { world.spawn((filler::C32(arch_idx), filler::C34(0), filler::C36(0), filler::C38(0), filler::C33(0), filler::C35(0), filler::C37(0), filler::C39(0))); }
                        }
                    }
                }
            }
        }

        world
    }

    pub fn position_only_system(mut query: bevy_ecs::prelude::Query<&mut Position>) {
        for mut pos in query.iter_mut() {
            pos.x += 1.0;
            pos.y += 1.0;
        }
    }
}

// ============================================================================
// Benchmark: Iterate entities with Position + Velocity (movement system)
// Matches 200 archetypes × 1000 entities = 200,000 entities
// ============================================================================

fn bench_iterate_movement(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_iterate_movement");
    group.sample_size(10); // Reduce sample size for expensive benchmarks

    group.bench_function("nimbus", |b| {
        let mut app = nimbus::setup_app();
        // Register system once - it will cache query state internally
        app.register_system(nimbus_ecs::SystemPriority::Update, nimbus::movement_system);
        // Warm up - first run populates the cache
        let _ = app.run();

        b.iter(|| {
            // Subsequent runs use cached archetype matching
            app.run().unwrap();
        });
    });

    group.bench_function("bevy", |b| {
        let mut world = bevy::setup_world();

        let mut system_state: bevy_ecs::system::SystemState<
            bevy_ecs::prelude::Query<(&mut bevy::Position, &bevy::Velocity)>,
        > = bevy_ecs::system::SystemState::new(&mut world);

        b.iter(|| {
            let mut query = system_state.get_mut(&mut world);
            for (mut pos, vel) in query.iter_mut() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Iterate entities with Position only
// Matches 400 archetypes × 1000 entities = 400,000 entities
// ============================================================================

fn bench_iterate_position_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_iterate_position");
    group.sample_size(10);

    // Both use scheduler/system for fair comparison
    group.bench_function("nimbus", |b| {
        let mut app = nimbus::setup_app();
        // Register system once - it will cache query state internally
        app.register_system(nimbus_ecs::SystemPriority::Update, nimbus::position_only_system);
        // Warm up - first run populates the cache
        let _ = app.run();

        b.iter(|| {
            // Subsequent runs use cached archetype matching
            app.run().unwrap();
        });
    });

    // Bevy also using schedule for fair comparison
    group.bench_function("bevy", |b| {
        let mut world = bevy::setup_world();
        let mut schedule = bevy_ecs::schedule::Schedule::default();
        schedule.add_systems(bevy::position_only_system);
        // Warm up
        schedule.run(&mut world);

        b.iter(|| {
            schedule.run(&mut world);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: World setup time (spawning 500k entities across 500 archetypes)
// ============================================================================

fn bench_world_setup(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_world_setup");
    group.sample_size(10);

    group.bench_function("nimbus", |b| {
        b.iter(|| {
            let world = nimbus::setup_world();
            black_box(world)
        });
    });

    group.bench_function("bevy", |b| {
        b.iter(|| {
            let world = bevy::setup_world();
            black_box(world)
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Cached system reruns (tests query archetype caching)
// 
// This benchmark measures the benefit of query state caching:
// - nimbus: uses registered system with cached QueryState
// - bevy: uses SystemState for equivalent caching
// 
// Both skip the archetype matching work on reruns.
// ============================================================================

fn bench_cached_system_rerun(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_cached_rerun");
    group.sample_size(100); // More samples since individual runs are faster

    group.bench_function("nimbus", |b| {
        let mut app = nimbus::setup_app();
        // Register the system once - it will cache query state internally
        app.register_system(nimbus_ecs::SystemPriority::Update, nimbus::movement_system);
        
        // Warm up - first run populates the cache
        let _ = app.run();

        b.iter(|| {
            // Subsequent runs use cached archetype matching
            app.run().unwrap();
        });
    });

    group.bench_function("bevy", |b| {
        let mut world = bevy::setup_world();

        // SystemState caches the query state (Bevy's equivalent)
        let mut system_state: bevy_ecs::system::SystemState<
            bevy_ecs::prelude::Query<(&mut bevy::Position, &bevy::Velocity)>,
        > = bevy_ecs::system::SystemState::new(&mut world);

        // Warm up
        {
            let mut query = system_state.get_mut(&mut world);
            for (mut pos, vel) in query.iter_mut() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        b.iter(|| {
            let mut query = system_state.get_mut(&mut world);
            for (mut pos, vel) in query.iter_mut() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: First run vs cached run comparison
// Shows the overhead saved by caching
// ============================================================================

fn bench_first_vs_cached(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_first_vs_cached");
    group.sample_size(10);

    // First run - no cache, must scan all archetypes
    group.bench_function("nimbus_first_run", |b| {
        b.iter_batched(
            || nimbus::setup_world(),
            |mut world| {
                // run_system creates a new FunctionSystem each time (no cache)
                world.run_system(nimbus::movement_system).unwrap();
            },
            criterion::BatchSize::LargeInput,
        );
    });

    // Cached run - system registered, uses cached query state
    group.bench_function("nimbus_cached_run", |b| {
        let mut app = nimbus::setup_app();
        app.register_system(nimbus_ecs::SystemPriority::Update, nimbus::movement_system);
        // Warm up cache
        let _ = app.run();

        b.iter(|| {
            app.run().unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Query creation only (no iteration)
// Isolates the cost of creating queries and finding matching archetypes
// ============================================================================

fn bench_query_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_query_creation");
    group.sample_size(100);

    // Direct query (uses component index, no caching)
    group.bench_function("direct_query_count", |b| {
        let mut world = nimbus::setup_world();
        
        b.iter(|| {
            world.query::<(&nimbus::Position, &nimbus::Velocity)>().iter().count()
        });
    });

    // run_system with the actual movement system
    group.bench_function("run_system_movement", |b| {
        let mut world = nimbus::setup_world();
        
        b.iter(|| {
            world.run_system(nimbus::movement_system).unwrap();
        });
    });

    // run_system with empty system (isolate system setup overhead)
    group.bench_function("run_system_empty", |b| {
        fn empty_system() {}
        let mut world = nimbus::setup_world();
        
        b.iter(|| {
            world.run_system(empty_system).unwrap();
        });
    });

    // run_system with count-only system
    group.bench_function("run_system_count_only", |b| {
        fn count_system(mut query: nimbus_ecs::Query<(&nimbus::Position, &nimbus::Velocity)>) {
            black_box(query.iter().count());
        }
        let mut world = nimbus::setup_world();
        
        b.iter(|| {
            world.run_system(count_system).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_iterate_movement,
    bench_iterate_position_only,
    bench_world_setup,
    bench_cached_system_rerun,
    bench_first_vs_cached,
    bench_query_creation,
);
criterion_main!(benches);
