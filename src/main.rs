#![allow(clippy::too_many_arguments, clippy::type_complexity)]

// TODO:
// - Animar cartas mejor, darles textura
// - Tomar cartas (basado en opciones del script)
// - Jugar las cartas
// - Menú de opciones y pantalla de cartas
// - Integración con fmod
// - Dibujar mejores texturas
// - Efectos especiales, polish, etc...

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
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

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
        .add_systems(PreStartup, (res_init, dialogue::res_init))
        .add_systems(Startup, (scene_init, dialogue::box_init, dialogue::card_init))
        .add_systems(Update, (dialogue::update, dialogue::card_update, dialogue::pick_card_update, candle_update, animation_post_init))
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

// Resource initialization
fn res_init(mut cmd : Commands) {
    // Perlin noise resource
    cmd.insert_resource(PerlinNoise(Perlin::new(1)));

    // Player
    cmd.spawn((
        SpatialBundle {
            transform : Transform::from_xyz(-5.5, 6.0, 15.0),
            ..default()
        },
        Player{}
    ));
}

// 3D Scene initalization
fn scene_init(mut cmd : Commands, assets : Res<AssetServer>, player : Query<Entity, With<Player>>) {
    // Main camera
    let cam = cmd.spawn((
        Camera3dBundle {
            ..default()
        },
        FogSettings {
            color : Color::rgba(0.01, 0.01, 0.01, 0.7),
            falloff : FogFalloff::Linear {
                start: 20.0,
                end: 35.0,
            },
            ..default()
        }
    )).id();
    cmd.entity(player.single()).push_children(&[cam]);

    // Player point light
    cmd.spawn(
        PointLightBundle {
            transform : Transform::from_xyz(-8.0, 5.2, 16.0),
            point_light : PointLight {
                color : Color::rgb(1.0, 0.7, 0.5),
                intensity : 500.,
                ..default()
            },
            ..default()
        }
    );

    // Load scene from gltf (exported from Blender)
    cmd.spawn(
        SceneBundle {
            scene : assets.load("models/escena.glb#Scene0"),
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
// Late startup systems

// Start playing the first animation
fn animation_post_init(animations : Res<Animations>, mut anim_player : Query<&mut AnimationPlayer>, mut done : Local<bool>) {
    if !*done {
        if let Ok(mut anim_player) = anim_player.get_single_mut() {
            anim_player.play(animations.0[0].clone_weak()).repeat();
            *done = true;
        }
    }
}

// ---
// Update systems

// Also animate the candle lighs with flicker
fn candle_update(time : Res<Time>, perlin : Res<PerlinNoise>, mut lights : Query<&mut PointLight>) {
    for mut light in lights.iter_mut().filter(|x| x.intensity < 500.) {
        light.intensity = 100. + 40. * perlin.0.get([3. * time.elapsed_seconds() as f64, 0.0]) as f32;
    }
}

