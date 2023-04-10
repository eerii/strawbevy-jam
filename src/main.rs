#![allow(clippy::too_many_arguments, clippy::type_complexity)]

// TODO:
// - Música
// - Mejorar menú
// - Efectos especiales, polish, etc...

mod yarn;
mod dialogue;

// ---

use yarn::YarnPlugin;

use bevy::{
    prelude::*,
    window::WindowResolution,
    render::{render_resource::TextureDescriptor, view::RenderLayers}, core_pipeline::clear_color::ClearColorConfig
};

use std::collections::HashMap;
use noise::{Perlin, NoiseFn};

// ---

const MENU_BACKGROUND : Color = Color::rgb(0.05, 0.12, 0.08);
const MENU_BUTTON_REGULAR : Color = Color::rgba(0., 0., 0., 0.2);
const MENU_BUTTON_HOVER : Color = Color::rgba(0.2, 0.5, 0.3, 0.05);

const LOOK_REMIE : Vec3 = Vec3::new(0.0, -0.2, -1.0);
const LOOK_NICO : Vec3 = Vec3::new(0.2, -0.2, -1.0);

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
            change_cam
                .run_if(resource_changed::<GameState>()),
            (check_loading, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Loading) )),
            (menu_update, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Menu) )),
            (dialogue::update, dialogue::card_update, dialogue::pick_card_update,
             dialogue::create_cards_update, dialogue::card_words_update,
             candle_update, remie_update, player_update, transparency_update, check_for_menu_update)
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Play) )),
        ))
        .run();
}

// ---
// Components

#[derive(Component)]
pub struct Player;

#[derive(Component)]
struct Remie;

#[derive(Component)]
enum CamId {
    Player,
    Menu
}

#[derive(Component)]
enum MenuButton {
    Start,
    Options
}

#[derive(Component)]
struct MenuNode;

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
pub struct StoryState{
    is_marco_here : bool,
    //is_remie_here : bool,
    //drink : Option<String>,
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

    // Story state
    cmd.insert_resource(StoryState{
        is_marco_here : false,
        //is_remie_here : true,
        //drink : None,
    });

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
    let menu_pass_layer = RenderLayers::layer(1);
    cmd.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(MENU_BACKGROUND)
            },
            camera : Camera {
                is_active : false,
                ..default()
            },
            ..default()
        },
        menu_pass_layer,
        CamId::Menu
    ));

    // Menu text style
    let button_style = TextStyle {
        font: props.font.clone(),
        font_size: 24.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };
    let title_style = TextStyle {
        font: props.font.clone(),
        font_size: 48.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };

    // Menu node
    cmd.spawn((
        NodeBundle {
            style : Style {
                size : Size::width(Val::Percent(100.0)),
                align_items : AlignItems::Center,
                justify_content : JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                gap : Size::new(Val::Auto, Val::Px(32.0)),
                ..default()
            },
            ..default()
        },
        menu_pass_layer,
        MenuNode
    ))
    .with_children(|parent| {
        parent.spawn(
            TextBundle::from_section("Working Title", title_style.clone())
        );

        let button_flex_style = Style {
            size : Size::new(Val::Px(150.0), Val::Px(65.0)),
            justify_content : JustifyContent::Center,
            align_items : AlignItems::Center,
            ..default()
        };

        parent.spawn((
            ButtonBundle {
                style : button_flex_style.clone(),
                background_color : MENU_BUTTON_REGULAR.into(),
                ..default()
            },
            MenuButton::Start
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section("Play", button_style.clone()));
        });

        parent.spawn((
            ButtonBundle {
                style : button_flex_style,
                background_color : MENU_BUTTON_REGULAR.into(),
                ..default()
            },
            MenuButton::Options
        )).with_children(|parent| {
            parent.spawn(TextBundle::from_section("Options", button_style));
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
        CamId::Player
    )).id();

    // Player point light
    let player_light = cmd.spawn(
        PointLightBundle {
            transform : Transform::from_xyz(-0.5, 0.0, 4.0),
            point_light : PointLight {
                color : MENU_BUTTON_REGULAR,
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
              mut cameras : Query<(&mut Camera, &CamId)>,
              mut node : Query<&mut Visibility, With<MenuNode>>) {
    for (mut cam, cam_id) in cameras.iter_mut() {
        match cam_id {
            CamId::Player => cam.is_active = matches!(*state, GameState::Play),
            CamId::Menu => cam.is_active = matches!(*state, GameState::Menu),
        }
    }
    node.iter_mut().for_each(|mut x| *x = match *state {
        GameState::Menu => Visibility::Visible,
        _ => Visibility::Hidden
    })
}

// ---
// Update systems

// Main menu
fn check_for_menu_update(mut state : ResMut<GameState>,
                         keyboard : Res<Input<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        *state = GameState::Menu;
    }
}

fn menu_update(mut state : ResMut<GameState>,
               mut buttons : Query<(&Interaction, &mut BackgroundColor, &MenuButton), Changed<Interaction>>) {
    for (inter, mut bg, button) in buttons.iter_mut() {
        match *inter {
            Interaction::Clicked => {
                match *button {
                    MenuButton::Start => {
                        *state = GameState::Play;
                    },
                    MenuButton::Options => {

                    }
                }
            },
            Interaction::Hovered => {
                *bg = MENU_BUTTON_HOVER.into();
            },
            Interaction::None => {
                *bg = MENU_BUTTON_REGULAR.into();
            },
        }
    }
}

// Animate Remie
fn remie_update(time : Res<Time>, mut remie : Query<&mut Transform, With<Remie>>) {
    if let Ok(mut trans) = remie.get_single_mut() {
        trans.translation.y = 3.5 + (time.elapsed_seconds() * 1.5).cos() * 0.05;
    }
}

// Animate player camera
fn player_update(time : Res<Time>,
                 story : Res<StoryState>,
                 mut player : Query<&mut Transform, With<Player>>,
                 mut lerp_time : Local<(f32, bool)>) {
    if let Ok(mut trans) = player.get_single_mut() {
        if story.is_marco_here != lerp_time.1 {
            *lerp_time = (0., story.is_marco_here);
        }

        if story.is_marco_here {
            *trans = trans.looking_at(trans.translation + LOOK_REMIE.lerp(LOOK_NICO, lerp_time.0), Vec3::Y);
        } else {
            *trans = trans.looking_at(trans.translation + LOOK_NICO.lerp(LOOK_REMIE, lerp_time.0), Vec3::Y);
        };

        if lerp_time.0 >= 1. { lerp_time.0 = 1.; }
        else { lerp_time.0 += time.delta_seconds() * 2.; }
    }
}

// Also animate the candle lighs with flicker
fn candle_update(time : Res<Time>, perlin : Res<PerlinNoise>, mut lights : Query<&mut PointLight>) {
    for mut light in lights.iter_mut().filter(|x| x.intensity < 500.) {
        light.intensity = 100. + 40. * perlin.0.get([3. * time.elapsed_seconds() as f64, 0.0]) as f32;
    }
}

// Add transparency to sprites
fn transparency_update(mut materials : ResMut<Assets<StandardMaterial>>, mut done : Local<bool>) {
    if !*done {
        for (_, mat) in materials.iter_mut() {
            mat.alpha_mode = AlphaMode::Mask(0.5);
        }
        *done = true;
    }
}