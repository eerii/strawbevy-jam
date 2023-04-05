// Dialogue system using the yarn spinner plugin for bevy

use super::{
    Player, TextResource,
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

// ---
// Constants

const CARD_PADDING : f32 = 0.15;
const CARD_SIZE : Extent3d = Extent3d { width: 256, height: 384, depth_or_array_layers: 1 };

// ---
// Components

#[derive(Component)]
pub struct DialogueBox;

enum CardStatus {
    Empty,
    Rendered,
    Done
}
impl Default for CardStatus {
    fn default() -> Self {
        CardStatus::Empty
    }
}

#[derive(Debug)]
struct CardRenderError;

#[derive(Component, Default)]
pub struct DialogueCard<'a> {
    status : CardStatus,
    text : &'a str,
    image : Handle<Image>,
    style : TextStyle,
    camera : Option<Entity>,
    text_renderer : Option<Entity>
}

impl<'a> DialogueCard<'a> {
    fn new(text : &'a str, image : Handle<Image>, style : TextStyle) -> DialogueCard<'a> {
        DialogueCard { text, image, style, ..default() }
    }

    fn render(&mut self, render_layer : u8, cmd : &mut Commands) -> Result<(), CardRenderError> {
        if let CardStatus::Rendered = self.status { return Err(CardRenderError) };

        let text_pass_layer = RenderLayers::layer(render_layer);
        // Camera to render the 2d text onto the card image
        self.camera = Some(cmd.spawn((
            Camera2dBundle {
                camera: Camera {
                    order: -1,
                    target: RenderTarget::Image(self.image.clone()),
                    ..default()
                },
                ..default()
            },
            text_pass_layer
        )).id());

        // Create image text
        self.text_renderer = Some(cmd.spawn((
            Text2dBundle {
                text : Text::from_section(self.text, self.style.clone()),
                transform : Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
            text_pass_layer
        )).id());

        self.status = CardStatus::Rendered;
        Ok(())
    }

    fn clean(&mut self, cmd : &mut Commands) -> Result<(), CardRenderError> {
        let CardStatus::Rendered = self.status else { return Err(CardRenderError) };

        // Now that the card has been rendered, delete the camera and text renderer
        if let Some(camera) = self.camera {
            cmd.entity(camera).despawn_recursive();
        }
        if let Some(text_renderer) = self.text_renderer {
            cmd.entity(text_renderer).despawn_recursive();
        }

        self.status = CardStatus::Done;
        Ok(())
    }
}

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

    // Text properties
    let texture_descriptor = TextureDescriptor {
        label: None,
        size: CARD_SIZE,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUsages::TEXTURE_BINDING
             | TextureUsages::COPY_DST
             | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[TextureFormat::Bgra8UnormSrgb],
    };
    let text_style = TextStyle {
        font : assets.load("fonts/dogicabold.ttf"),
        font_size : 16.0,
        color : Color::WHITE,
    };

    // Create test dialogue cards
    let words = ["hey"];
    for w in words {
        let mut image = Image { texture_descriptor : texture_descriptor.clone(), ..default() };
        image.resize(CARD_SIZE);
        let image_handle = images.add(image);

        cmd.spawn((
            PbrBundle {
                mesh: card_mesh.clone(),
                material: materials.add(StandardMaterial{base_color_texture : Some(image_handle.clone()), ..default()}),
                transform: Transform::from_scale(Vec3::splat(0.)),
                ..default()
            },
            DialogueCard::new(w, image_handle.clone(), text_style.clone()),
        ));
    }
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
pub fn card_update(mut cmd : Commands,
                   time : Res<Time>,
                   mut player : Query<&mut Transform, With<Player>>,
                   mut cards : Query<(&mut DialogueCard<'static>, &mut Transform), Without<Player>>,
                   mut render_layer : Local<u8>) {
    // Obtain the player transformation
    if let Ok(mut player_trans) = player.get_single_mut() {
        //TODO: Delete this
        *player_trans = player_trans.looking_at(Vec3::new(-5.5 + time.elapsed_seconds().sin(), 3.0, 0.0), Vec3::Y);

        // Update the cards
        let n = cards.iter().count();
        for (i, (mut card, mut trans)) in cards.iter_mut().enumerate() {
            match card.status {
                CardStatus::Empty => {
                    *render_layer += 1;
                    card.render(*render_layer, &mut cmd).unwrap();
                },
                CardStatus::Rendered => {
                    card.clean(&mut cmd).unwrap();
                    trans.scale = Vec3::splat(1.);
                },
                CardStatus::Done => {
                    let off = CARD_PADDING * i as f32 - CARD_PADDING * ((n-1) as f32 / 2.0);
                    trans.translation = player_trans.translation + player_trans.rotation.mul_vec3(Vec3::new(off, -0.35 - off.abs() * 0.1, -1.0 - 0.01 * i as f32));
                    trans.rotation = player_trans.rotation.clone()
                        .mul_quat(Quat::from_rotation_z(-off * 0.5));
                }
            }
        }
    }
}
