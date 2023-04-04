#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod yarn;
mod dialogue;

// ---

use yarn::YarnPlugin;

use bevy::{
    prelude::*,
    window::WindowResolution
};

use noise::{Perlin, NoiseFn};

// ---
// App

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Strawbevy Jam".to_string(),
                resolution: WindowResolution::new(800., 800.),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(YarnPlugin)
        .add_systems(Startup, (init, scene_init, dialogue::box_init, dialogue::card_init))
        .add_systems(Update, (dialogue::update, dialogue::card_update, animation_update))
        .run();
}

// ---
// Components

#[derive(Component)]
pub struct Player;

// ---
// Resources

#[derive(Resource)]
struct Animations(Vec<Handle<AnimationClip>>);

#[derive(Resource)]
struct PerlinNoise(Perlin);

// ---
// Startup systems

// General initialization
fn init(mut cmd : Commands) {
    // Perlin noise resource
    cmd.insert_resource(PerlinNoise(Perlin::new(1)));
}

// 3D Scene initalization
fn scene_init(mut cmd : Commands, assets : Res<AssetServer>) {
    // Main camera
    cmd.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-6.0, 6.0, 15.0)
                .looking_at(Vec3::new(-5.5, 3.0, 0.0), Vec3::Y),
            ..default()
        },
        Player{}
    ));

    // Load scene from gltf (exported from Blender)
    cmd.spawn(
        SceneBundle {
            scene: assets.load("models/escena.glb#Scene0"),
            ..default()
        }
    );

    // Load animations
    cmd.insert_resource(
        Animations(vec![
            assets.load("models/escena.glb#Animation0")
        ])
    );
}

// ---
// Update systems

// Start playing the animation from the scene after it is loaded
// Also animate the candle lights with flicker
fn animation_update(animations : Res<Animations>,
                    time : Res<Time>,
                    perlin : Res<PerlinNoise>,
                    mut anim_player : Query<&mut AnimationPlayer>,
                    mut lights : Query<&mut PointLight>,
                    mut is_anim_init : Local<bool>) {
    if !*is_anim_init {
        if let Ok(mut anim_player) = anim_player.get_single_mut() {
            anim_player.play(animations.0[0].clone_weak()).repeat();
            *is_anim_init = true;
        }
    }

    for mut light in lights.iter_mut().filter(|x| x.intensity < 500.) {
        light.intensity = 100. + 40. * perlin.0.get([3. * time.elapsed_seconds() as f64, 0.0]) as f32;
    }
}
