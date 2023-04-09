#![allow(clippy::too_many_arguments, clippy::type_complexity)]

// TODO:
// - Menú de opciones y pantalla de cartas
// - Música
// - Load asset screen and main menu
// - Efectos especiales, polish, etc...

mod yarn;
mod dialogue;

// ---

use yarn::YarnPlugin;

use bevy::{
    prelude::*,
    window::WindowResolution,
    render::render_resource::TextureDescriptor
};

use std::collections::HashMap;
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
        .insert_resource(GameState::default())
        .insert_resource(AssetsLoading::default())
        .add_systems(PreStartup, (res_init, dialogue::res_init))
        .add_systems(Startup, (menu_init, scene_init, dialogue::box_init))
        .add_systems(Update, (
            change_cam,
            (check_loading, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Loading) )),
            (menu_update, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Loading) )),
            (dialogue::update, dialogue::card_update, dialogue::pick_card_update,
             dialogue::create_cards_update, dialogue::card_words_update,
             candle_update, remie_update, player_update, transparency_update)
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Play) )),
        ))
        .run();
}

// ---
// Components

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCam;

#[derive(Component)]
pub struct MenuCam;

#[derive(Component)]
struct Remie;

// ---
// Resources

#[derive(Resource, Default)]
pub enum GameState{
    #[default]
    Loading,
    Menu,
    Play,
}

#[derive(Resource)]
pub struct Props {
    box_mesh : Handle<Mesh>,
    box_style : HashMap<&'static str, TextStyle>,
    box_background : Handle<Image>,
    card_mesh : Handle<Mesh>,
    card_style : HashMap<&'static str, TextStyle>,
    card_texture_descriptor : TextureDescriptor<'static>,
    card_background : Handle<Image>,
    font : Handle<Font>,
}

#[derive(Resource, Default)]
pub struct AssetsLoading(Vec<HandleUntyped>);

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

// Menu initialization
fn menu_init(mut cmd : Commands, props : Res<Props>) {
    // Menú camera
    cmd.spawn((
        Camera2dBundle {
            camera : Camera {
                is_active : false,
                ..default()
            },
            ..default()
        },
        MenuCam{}
    ));

    // Menu text style
    let style = TextStyle {
        font: props.font.clone(),
        font_size: 24.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };

    // Menu node
    cmd.spawn(
        NodeBundle {
            style: Style {
                size: Size::width(Val::Percent(100.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        }
    )
    .with_children(|parent| {
        parent.spawn(
            ButtonBundle {
                style: Style {
                    size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::rgb(0.1, 0.1, 0.1).into(),
                ..default()
            }
        ).with_children(|parent| {
            parent.spawn(TextBundle::from_section("Button", style.clone()));
        });
    });
}

// 3D Scene initalization
fn scene_init(mut cmd : Commands,
              assets : Res<AssetServer>,
              mut loading : ResMut<AssetsLoading>,
              mut meshes: ResMut<Assets<Mesh>>,
              mut materials: ResMut<Assets<StandardMaterial>>,
              player : Query<Entity, With<Player>>) {
    // Main camera
    let player_cam = cmd.spawn((
        Camera3dBundle {
            camera : Camera {
                is_active : false,
                ..default()
            },
            ..default()
        },
        FogSettings {
            color : Color::rgba(0.01, 0.01, 0.01, 0.7),
            falloff : FogFalloff::Linear {
                start: 20.0,
                end: 35.0,
            },
            ..default()
        },
        PlayerCam{}
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
    let remie = assets.load("textures/remie.png");
    loading.0.push(remie.clone_untyped());
    cmd.spawn((
        PbrBundle {
            mesh : meshes.add(Mesh::from(shape::Quad::new(Vec2::new(3.0, 4.5)))),
            material : materials.add(StandardMaterial {
                base_color_texture : Some(remie),
                ..default()
            }),
            transform : Transform::from_xyz(-5.4, 3.5, 4.0),
            ..default()
        },
        Remie {}
    ));

    // Load scene from gltf (exported from Blender)
    let scene = assets.load("models/escena.glb#Scene0");
    loading.0.push(scene.clone_untyped());
    cmd.spawn(SceneBundle { scene, ..default() });
}

// ---
// Load systems

// Check if loading is finished
fn check_loading(mut cmd : Commands, mut state : ResMut<GameState>, assets : Res<AssetServer>, loading : Res<AssetsLoading>) {
    use bevy::asset::LoadState;
    match assets.get_group_load_state(loading.0.iter().map(|x| x.id())) {
        LoadState::Failed => todo!(),
        LoadState::Loaded => {
            cmd.remove_resource::<AssetsLoading>();
            *state = GameState::Menu;
        },
        _ => ()
    }
}

// Change the active camera (menu / player)
fn change_cam(state : Res<GameState>,
              mut player_cam : Query<&mut Camera, With<PlayerCam>>,
              mut menu_cam : Query<&mut Camera, (With<MenuCam>, Without<PlayerCam>)>) {
    if let Ok(mut cam) = player_cam.get_single_mut() {
        cam.is_active = matches!(*state, GameState::Play);
    }
    if let Ok(mut cam) = menu_cam.get_single_mut() {
        cam.is_active = matches!(*state, GameState::Menu);
    }
}

// ---
// Update systems

// Main menu
fn menu_update() {

}

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

// Add transparency to sprites
fn transparency_update(mut materials : ResMut<Assets<StandardMaterial>>, done : Local<bool>) {
    if !*done {
        for (_, mat) in materials.iter_mut() {
            mat.alpha_mode = AlphaMode::Mask(0.5);
        }
    }
}