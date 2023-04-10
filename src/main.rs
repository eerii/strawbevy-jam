#![allow(clippy::too_many_arguments, clippy::type_complexity)]

// TODO:
// - Música
// - Text 2 speech ally
// - Luces cambian con ansiedad
// - Expresiones faciales
// - The end (finish and start over)
// - Mejorar menú
// - Efectos especiales, polish, etc...

mod yarn;
mod dialogue;

// ---

use yarn::YarnPlugin;

use bevy::{
    prelude::*,
    window::WindowResolution,
    render::{render_resource::TextureDescriptor, view::RenderLayers},
    core_pipeline::clear_color::ClearColorConfig
};
use bevy_pkv::PkvStore;

use std::collections::HashMap;
use noise::{Perlin, NoiseFn};

// ---

pub const NUM_ENDINGS : usize = 5;

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
        .insert_resource(PersistentStorage(PkvStore::new("koala", "strawbevyjam")))
        .add_systems(PreStartup, (res_init, dialogue::res_init))
        .add_systems(Startup, (menu_init, scene_init, dialogue::box_init))
        .add_systems(Update, (
            change_cam
                .run_if(resource_changed::<GameState>()),
            change_endings
                .run_if(resource_changed::<StoryState>()),
            (check_loading, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Loading) )),
            (menu_update, )
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Menu) )),
            (dialogue::update, dialogue::card_update, dialogue::pick_card_update,
             dialogue::create_cards_update, dialogue::card_words_update,
             candle_update, character_update, player_update, transparency_update, check_for_menu_update)
                .run_if(resource_exists::<GameState>().and_then(|state : Res<GameState>| matches!(*state, GameState::Play) )),
        ))
        .run();
}

// ---
// Components

#[derive(Component)]
pub struct Player;

#[derive(Component)]
enum Character {
    Remie,
    Marco
}

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

#[derive(Component)]
struct MenuEndings;

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
    is_remie_here : bool,
    endings : [bool; NUM_ENDINGS],
    selected_options : HashMap<u64, Vec<String>>,
    current_question : u64
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
    drink_textures : [Handle<Image>; 3],
    font : Handle<Font>,
}

#[derive(Resource, Default)]
pub struct AssetsLoading(Vec<HandleUntyped>);

#[derive(Resource)]
struct PerlinNoise(Perlin);

#[derive(Resource)]
pub struct PersistentStorage(PkvStore);

// ---
// Startup systems

// Resource initialization
fn res_init(mut cmd : Commands, storage : Res<PersistentStorage>) {
    // Perlin noise resource
    cmd.insert_resource(PerlinNoise(Perlin::new(1)));

    // Persistent storage
    let mut endings : [bool; NUM_ENDINGS] = [false; NUM_ENDINGS];
    let mut selected_options : HashMap<u64, Vec<String>> = HashMap::new();
    if let Ok(unlocked) = storage.0.get::<[bool; NUM_ENDINGS]>("unlocked_endings") {
        endings = unlocked;
    }
    if let Ok(options) = storage.0.get::<HashMap<u64, Vec<String>>>("selected_options") {
        selected_options = options;
    }

    // Story state
    cmd.insert_resource(StoryState{
        is_marco_here : false,
        is_remie_here : true,
        endings,
        selected_options,
        current_question : 0,
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
    let small_style = TextStyle {
        font: props.font.clone(),
        font_size: 16.0,
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
            TextBundle::from_section("Working Title", title_style)
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

        parent.spawn((
            TextBundle::from_section(format!("Discovered 0/{} endings", NUM_ENDINGS), small_style),
            MenuEndings{}
        ));
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
    cmd.entity(player.single()).push_children(&[player_cam]);

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
        Character::Remie
    ));

    // Marco character
    let marco = assets.load("textures/marco.png");
    loading.0.push(marco.clone_untyped());
    cmd.spawn((
        PbrBundle {
            mesh : meshes.add(Mesh::from(shape::Quad::new(Vec2::new(3.0, 4.5)))),
            material : materials.add(StandardMaterial {
                base_color_texture : Some(marco),
                ..default()
            }),
            transform : Transform::from_xyz(-1.0, 3.5, 5.0).with_rotation(Quat::from_rotation_y(-0.3)),
            ..default()
        },
        Character::Marco
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

// Update the endings in the menu
fn change_endings(story : Res<StoryState>, mut text : Query<&mut Text, With<MenuEndings>>) {
    if let Ok(mut text) = text.get_single_mut() {
        let endings = story.endings.iter().filter(|x| **x).count();
        text.sections[0].value = format!("Discovered {}/{} endings", endings, NUM_ENDINGS);
    }
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

// Animate Characters
fn character_update(time : Res<Time>, story : Res<StoryState>, mut characters : Query<(&mut Transform, &mut Visibility, &Character)>) {
    for (mut trans, mut visible, character) in characters.iter_mut() {
        trans.translation.y = 3.5 + (time.elapsed_seconds() * if let Character::Remie = character {1.5} else {1.8}).cos() * 0.05;
        *visible = match character {
            Character::Remie => if story.is_remie_here { Visibility::Visible } else { Visibility::Hidden },
            Character::Marco => if story.is_marco_here { Visibility::Visible } else { Visibility::Hidden },
        }
    }
}

// Smoothstep
pub fn smoothstep(x : f32, a : f32, b : f32) -> f32 {
    let t = (x - a) / (b - a);
    if t < 0.0 { 0.0 }
    else if t > 1.0 { 1.0 }
    else { t * t * (3.0 - 2.0 * t) }
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

        let head_wobble = Vec3::new(if lerp_time.0 >= 1. { time.elapsed_seconds().sin() * 0.005 } else { 0. }, 0., 0.);

        if story.is_marco_here {
            *trans = trans.looking_at(trans.translation + head_wobble + LOOK_REMIE.lerp(LOOK_NICO, smoothstep(lerp_time.0, 0., 1.)), Vec3::Y);
        } else {
            *trans = trans.looking_at(trans.translation + head_wobble + LOOK_NICO.lerp(LOOK_REMIE, smoothstep(lerp_time.0, 0., 1.)), Vec3::Y);
        };

        if lerp_time.0 >= 1. { lerp_time.0 = 1.; }
        else { lerp_time.0 += time.delta_seconds() * 2.; }
    }
}

// Also animate the candle lighs with flicker
fn candle_update(time : Res<Time>, perlin : Res<PerlinNoise>, mut lights : Query<&mut PointLight>) {
    for mut light in lights.iter_mut().filter(|x| x.intensity < 800.) {
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