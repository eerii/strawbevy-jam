#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod dialogue;
use dialogue::*;

use bevy::{
    prelude::*,
    /*render::{
        render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers, camera::RenderTarget
    }*/
};

// ---
// App

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DialoguePlugin)
        .add_systems(Startup, (init, dialogue_init))
        .add_systems(Update, dialogue_update)
        .run();
}

// ---
// Components


// ---
// Startup systems

// General initialization
fn init(mut cmd : Commands, assets : Res<AssetServer>) {
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

    // Main camera
    cmd.spawn(Camera2dBundle::default());
    
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
        DialogueBox {}
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


