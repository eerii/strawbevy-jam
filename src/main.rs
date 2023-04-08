#![allow(clippy::too_many_arguments, clippy::type_complexity)]

// TODO:
// - Menú de opciones y pantalla de cartas
// - Música
// - Dibujar mejores texturas, hacer radio, vela, bebidas
// - Load asset screen and main menu
// - Efectos especiales, polish, etc...

mod yarn;
mod dialogue;

// ---

use yarn::YarnPlugin;

use bevy::{
    prelude::*,
    window::WindowResolution,
};

use noise::{Perlin, NoiseFn};

// ---
// App

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Strawbevy Jam".to_string(),
                    resolution: WindowResolution::new(800., 800.),
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
        )
        .add_plugin(YarnPlugin)
        .add_systems(PreStartup, (res_init, dialogue::res_init))
        .add_systems(Startup, (scene_init, dialogue::box_init))
        .add_systems(Update, (dialogue::update, dialogue::card_update, dialogue::pick_card_update,
                              dialogue::create_cards_update, dialogue::card_words_update,
                              candle_update, remie_update, player_update))
        .run();
}

// ---
// Components

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct Remie;

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
fn scene_init(mut cmd : Commands,
              assets : Res<AssetServer>,
              mut meshes: ResMut<Assets<Mesh>>,
              mut materials: ResMut<Assets<StandardMaterial>>,
              player : Query<Entity, With<Player>>) {
    // Main camera
    let player_cam = cmd.spawn((
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

    // Player point light
    let player_light = cmd.spawn(
        PointLightBundle {
            transform : Transform::from_xyz(-0.5, 0.0, 4.0),
            point_light : PointLight {
                color : Color::rgb(0.8, 0.5, 0.3),
                intensity : 500.,
                ..default()
            },
            ..default()
        }
    ).id();

    cmd.entity(player.single()).push_children(&[player_cam, player_light]);

    // Remie character
    cmd.spawn((
        PbrBundle {
            mesh : meshes.add(Mesh::from(shape::Quad::new(Vec2::new(3.0, 4.5)))),
            material : materials.add(StandardMaterial {
                base_color_texture : Some(assets.load("textures/remie.png")),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }),
            transform : Transform::from_xyz(-5.4, 3.5, 4.0),
            ..default()
        },
        Remie {}
    ));

    // Load scene from gltf (exported from Blender)
    cmd.spawn(
        SceneBundle {
            scene : assets.load("models/escena.glb#Scene0"),
            ..default()
        }
    );
}

// ---
// Update systems

// Animate Remie
fn remie_update(time : Res<Time>, mut remie : Query<&mut Transform, With<Remie>>) {
    if let Ok(mut trans) = remie.get_single_mut() {
        trans.translation.y = 3.5 + (time.elapsed_seconds() * 1.5).cos() * 0.05;
    }
}

// Animate player camera
fn player_update(time : Res<Time>, mut player : Query<&mut Transform, With<Player>>) {
    if let Ok(mut trans) = player.get_single_mut() {
        *trans = trans.looking_at(trans.translation + Vec3::new(time.elapsed_seconds().sin() * 0.02, -0.2, -1.), Vec3::Y);
    }
}

// Also animate the candle lighs with flicker
fn candle_update(time : Res<Time>, perlin : Res<PerlinNoise>, mut lights : Query<&mut PointLight>) {
    for mut light in lights.iter_mut().filter(|x| x.intensity < 500.) {
        light.intensity = 100. + 40. * perlin.0.get([3. * time.elapsed_seconds() as f64, 0.0]) as f32;
    }
}

