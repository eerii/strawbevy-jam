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
        .add_systems(Startup, (scene_init, text_init, dialogue_init))
        .add_systems(Update, dialogue_update)
        .run();
}

// ---
// Components

#[derive(Component)]
struct FpsText {}

// ---
// Startup systems

// 3D Scene initalization
fn scene_init(mut cmd : Commands, assets : Res<AssetServer>) {
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
}

// General initialization
fn text_init(mut cmd : Commands, assets : Res<AssetServer>) {
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
    for i in 0..3 {
        cmd.spawn((
            Text2dBundle {
                text : Text::from_section("", text_style.clone()),
                transform : Transform::from_xyz(250.0 * (i as f32 - 1.0), -100.0, 0.0),
                ..default()
            },
            DialogueOption{}
        ));
    }
}

// ---
// Update systems
