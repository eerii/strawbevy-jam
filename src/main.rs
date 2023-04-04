#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod dialogue;
use dialogue::*;

use bevy::{
    prelude::*,
    core_pipeline::clear_color::ClearColorConfig,
    window::WindowResolution,
    /*render::{
        render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers, camera::RenderTarget
    }*/
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
        .add_plugin(DialoguePlugin)
        .add_systems(Startup, (init, scene_init, dialoguebox_init))
        .add_systems(Update, (dialogue_update, animation_update))
        .run();
}

// ---
// Components

#[derive(Component)]
struct DialogueBox {}

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
fn scene_init(mut cmd : Commands, assets : Res<AssetServer>, mut yarn : ResMut<DialogueManager>) {
    // Main camera
    cmd.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-6.0, 6.0, 15.0)
            .looking_at(Vec3::new(-5.5, 3.0, 0.0), Vec3::Y),
        ..default()
    });

    // Load scene from gltf (exported from Blender)
    cmd.spawn(SceneBundle {
        scene: assets.load("models/escena.glb#Scene0"),
        ..default()
    });

    // Load animations
    cmd.insert_resource(Animations(vec![
        assets.load("models/escena.glb#Animation0")
    ]));

    // Load dialogue
    yarn.load("test", &assets);
}

// Dialogue box initialization
fn dialoguebox_init(mut cmd : Commands, assets : Res<AssetServer>) {
    // Render text camera
    /*let size = Extent3d { width: 512, height: 512, ..default() };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
        },
        ..default()
    };
    image.resize(size);
    let text_image_target = images.add(image);

    let text_pass_layer = RenderLayers::layer(1); // Add component to objects you want to render in this pass
    cmd.spawn((Camera2dBundle {
        camera: Camera {
            order: -1,
            target: RenderTarget::Image(text_image_target.clone()),
            ..default()
        },
        ..default()
    }, text_pass_layer));*/

    // Dialogue text camera
    cmd.spawn(Camera2dBundle{
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::None
        },
        camera: Camera {
            order: 1,
            ..default()
        },
        ..default()
    });
    
    // Dialogue box
    let text_style = TextStyle {
        font : assets.load("fonts/dogicabold.ttf"),
        font_size : 16.0,
        color : Color::WHITE,
    };
    cmd.spawn((
        Text2dBundle {
            text : Text::from_section("", text_style.clone()),
            transform : Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        DialogueBox{}
    ));
}

// ---
// Update systems

// Handle the changes in dialogue updates
fn dialogue_update(keyboard : Res<Input<KeyCode>>, 
                   mut yarn : ResMut<DialogueManager>,
                   mut asset_runner : ResMut<Assets<DialogueRunner>>,
                   asset_lines : ResMut<Assets<DialogueLines>>,
                   mut dialogue_box : Query<&mut Text, With<DialogueBox>>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let (runner, lines) = match get_dialogue_components(&yarn, &mut asset_runner, &asset_lines) {
        None => return,
        Some(v) => v
    };
    
    // For now just use the first response when having options
    // This will be handled by selecting a card
    if yarn.waiting_response {
        println!("TODO: SELECT CARD");
        runner.select_option(0).unwrap();
        yarn.waiting_response = false;
    }

    // Check if the dialogue is paused and if the user is continuing
    if keyboard.just_pressed(KeyCode::Space) {
        yarn.waiting_continue = false;
    }
    if yarn.waiting_continue || yarn.waiting_response {
        return;
    }

    // Update the dialogue with the options
    if let Ok(Some(dialogue)) = runner.execute(&mut yarn.storage) {
        match dialogue {
            ExecutionOutput::Line(line) => {
                let new_line = lines.line(&line).unwrap();
                dialogue_box.single_mut().sections[0].value = new_line;
                yarn.waiting_continue = true;
            },
            ExecutionOutput::Options(_opts) => {
                yarn.waiting_response = true;
            },
            ExecutionOutput::Command(cmd) => {
                println!("todo: {:?}", cmd);
            },
            ExecutionOutput::Function(function) => {
                let output = yarn_spinner::handle_default_functions(&function);
                runner.return_function(output.unwrap().unwrap()).unwrap();
            }
        }
    }
}

// Start playing the animation from the scene after it is loaded
// Also animate the candle lights with flicker
fn animation_update(animations : Res<Animations>,
                    time : Res<Time>,
                    perlin : Res<PerlinNoise>,
                    mut player : Query<&mut AnimationPlayer>,
                    mut lights : Query<&mut PointLight>,
                    mut is_anim_init : Local<bool>) {
    if !*is_anim_init {
        if let Ok(mut player) = player.get_single_mut() {
            player.play(animations.0[0].clone_weak()).repeat();
            *is_anim_init = true;
        }
    }

    for mut light in lights.iter_mut().filter(|x| x.intensity < 500.) {
        light.intensity = 100. + 40. * perlin.0.get([3. * time.elapsed_seconds() as f64, 0.0]) as f32;
    }
}
