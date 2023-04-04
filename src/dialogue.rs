// Dialogue system using the yarn spinner plugin for bevy

use super::{
    Player,
    yarn::*
};
use bevy::{
    prelude::*,
    core_pipeline::clear_color::ClearColorConfig,
    render::{
        render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers, camera::RenderTarget
    },
};

const CARD_PADDING : f32 = 0.15;

// ---
// Components

#[derive(Component)]
pub struct DialogueBox;

#[derive(Component)]
pub struct DialogueCard;

// ---
// Startup systems

// Dialogue box initialization
pub fn box_init(mut cmd : Commands, assets : Res<AssetServer>) { 
    // Dialogue text camera
    cmd.spawn(
        Camera2dBundle{
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None
            },
            camera: Camera {
                order: 1,
                ..default()
            },
            ..default()
        }
    );
    
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

pub fn card_init(mut cmd : Commands,
                 assets : Res<AssetServer>,
                 mut yarn : ResMut<YarnManager>,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut images: ResMut<Assets<Image>>,
                 mut materials: ResMut<Assets<StandardMaterial>>) {
    // Load dialogue
    yarn.load("test", &assets);

    // Create plane mesh
    let card_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(0.2, 0.3))));

    // Create image target
    let size = Extent3d { width: 256, height: 384, ..default() };
    let texture_descriptor = TextureDescriptor {
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
    };

    let mut image = Image { texture_descriptor, ..default() };
    image.resize(size);
    let text_image_target = images.add(image);

    // Create image text
    let text_pass_layer = RenderLayers::layer(1);
    let text_style = TextStyle {
        font : assets.load("fonts/dogicabold.ttf"),
        font_size : 16.0,
        color : Color::WHITE,
    };
    cmd.spawn((
        Text2dBundle {
            text : Text::from_section("test", text_style.clone()),
            transform : Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        text_pass_layer
    ));


    // Create test dialogue cards
    cmd.spawn((
        PbrBundle {
            mesh: card_mesh.clone(),
            material: materials.add(StandardMaterial{base_color_texture: Some(text_image_target.clone()), ..default()}),
            ..default()
        },
        DialogueCard {},
    ));
    cmd.spawn((
        PbrBundle {
            mesh: card_mesh.clone(),
            material: materials.add(StandardMaterial{base_color: Color::rgb(1., 1., 0.), ..default()}),
            ..default()
        },
        DialogueCard {},
    ));
    cmd.spawn((
        PbrBundle {
            mesh: card_mesh.clone(),
            material: materials.add(StandardMaterial{base_color: Color::rgb(0., 1., 1.), ..default()}),
            ..default()
        },
        DialogueCard {},
    ));

    // Render text camera
    cmd.spawn((
        Camera2dBundle {
            camera: Camera {
                order: -1,
                target: RenderTarget::Image(text_image_target),
                ..default()
            },
            ..default()
        },
        text_pass_layer
    ));
}

// ---
// Update systems

// Handle the changes in dialogue updates
pub fn update(keyboard : Res<Input<KeyCode>>, 
              mut yarn : ResMut<YarnManager>,
              mut asset_runner : ResMut<Assets<YarnRunnerAsset>>,
              asset_lines : Res<Assets<YarnLinesAsset>>,
              mut dialogue_box : Query<&mut Text, With<DialogueBox>>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let (runner, lines) = match get_yarn_components(&yarn, &mut asset_runner, &asset_lines) {
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

// Updates the cards position and attributes
pub fn card_update(time : Res<Time>,
                   mut player : Query<&mut Transform, With<Player>>,
                   mut cards : Query<&mut Transform, (Without<Player>, With<DialogueCard>)>) {
    // Obtain the player transformation
    if let Ok(mut player_trans) = player.get_single_mut() {
        *player_trans = player_trans.looking_at(Vec3::new(-5.5 + time.elapsed_seconds().sin(), 3.0, 0.0), Vec3::Y);

        // Update the cards
        let n = cards.iter().count();
        for (i, mut trans) in cards.iter_mut().enumerate() {
            let off = CARD_PADDING * i as f32 - CARD_PADDING * ((n-1) as f32 / 2.0);
            trans.translation = player_trans.translation + player_trans.rotation.mul_vec3(Vec3::new(off, -0.35 - off.abs() * 0.1, -1.0 - 0.01 * i as f32));
            trans.rotation = player_trans.rotation.clone()
                .mul_quat(Quat::from_rotation_z(-off * 0.5));
        }
    }
}
