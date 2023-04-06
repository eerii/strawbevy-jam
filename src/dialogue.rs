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

// ---
// Constants

const CARD_PADDING : f32 = 0.16;
const CARD_SIZE : Extent3d = Extent3d { width: 256, height: 384, depth_or_array_layers: 1 };
const CARD_LERP_TIME : f32 = 0.2;
#[cfg(target_arch = "wasm32")]
const CARD_RENDER_FRAME_WAIT : u8 = 16;
#[cfg(not(target_arch = "wasm32"))]
const CARD_RENDER_FRAME_WAIT : u8 = 4;

// ---
// Resources

#[derive(Resource)]
pub struct Props {
    card_mesh : Handle<Mesh>,
    box_style : TextStyle,
    card_style : TextStyle,
    card_texture_descriptor : TextureDescriptor<'static>,
}

// ---
// Components

#[derive(Component)]
pub struct DialogueBox;

#[derive(Default, Debug)]
enum CardStatus {
    #[default]
    Empty,
    Rendered,
    Done
}

#[derive(Debug)]
struct CardRenderError;

#[derive(Component, Default, Debug)]
pub struct DialogueCard<'a> {
    status : CardStatus,
    text : &'a str,
    image : Handle<Image>,
    style : TextStyle,
    camera : Option<Entity>,
    text_renderer : Option<Entity>,
    render_layer : Option<u8>,
    ready_counter : u8,
    previous_trans : Transform,
    target_trans : Transform,
    lerp_time : f32,
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
        self.render_layer = Some(render_layer);
        self.ready_counter = CARD_RENDER_FRAME_WAIT;
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
        self.render_layer = None;
        Ok(())
    }
}

// ---
// Startup systems

// Resource initalization
pub fn res_init(mut cmd : Commands,
                assets : Res<AssetServer>,
                mut meshes: ResMut<Assets<Mesh>>) {
    // Create plane mesh
    let card_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(0.2, 0.3))));

    // Dialogue text styles
    let font = assets.load("fonts/dogicabold.ttf");
    let box_style = TextStyle {
        font : font.clone(),
        font_size : 16.0,
        color : Color::WHITE,
    };
    let card_style = TextStyle {
        font,
        font_size : 16.0,
        color : Color::BLACK,
    };

    // Card texture properties
    let card_texture_descriptor = TextureDescriptor {
        label: None,
        size: CARD_SIZE,
        dimension: TextureDimension::D2,
        format: TextureFormat::R8Unorm,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUsages::TEXTURE_BINDING
             | TextureUsages::COPY_DST
             | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[TextureFormat::R8Unorm],
    };

    // Save state
    cmd.insert_resource(Props { card_mesh, box_style, card_style, card_texture_descriptor });
}

// Dialogue box initialization
pub fn box_init(mut cmd : Commands, props : Res<Props>) { 
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
    cmd.spawn((
        Text2dBundle {
            text : Text::from_section("", props.box_style.clone()),
            transform : Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        DialogueBox{}
    ));
}

pub fn card_init(mut cmd : Commands,
                 props : Res<Props>,
                 assets : Res<AssetServer>,
                 mut yarn : ResMut<YarnManager>,
                 mut images: ResMut<Assets<Image>>,
                 mut materials: ResMut<Assets<StandardMaterial>>,
                 player : Query<Entity, With<Player>>) {
    // Load dialogue
    yarn.load("dialogue", &assets);
 
    // Create test cards
    let words = ["hey", "hi", "hello"];
    for word in words.iter() {
        create_card(word, player.single(), &mut cmd, &props, &mut images, &mut materials);
    }
}

fn create_card(word : &'static str,
               player : Entity,
               cmd : &mut Commands,
               props : &Res<Props>,
               images : &mut ResMut<Assets<Image>>,
               materials: &mut ResMut<Assets<StandardMaterial>>) -> Entity { 
    // Create dialogue card
    let mut image = Image { texture_descriptor : props.card_texture_descriptor.clone(), ..default() };
    image.resize(CARD_SIZE);
    let image_handle = images.add(image);

    let card = cmd.spawn((
        PbrBundle {
            mesh: props.card_mesh.clone(),
            material: materials.add(StandardMaterial{base_color_texture : Some(image_handle.clone()), ..default()}),
            transform: Transform::from_xyz(0., -1., -1.),
            ..default()
        },
        DialogueCard::new(word, image_handle, props.card_style.clone()),
    )).id();

    cmd.entity(player).push_children(&[card]);
    card
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

// Smoothstep
fn smoothstep(x : f32, a : f32, b : f32) -> f32 {
    let t = (x - a) / (b - a);
    if t < 0.0 { 0.0 }
    else if t > 1.0 { 1.0 }
    else { t * t * (3.0 - 2.0 * t) }
}

// Updates the cards position and attributes
pub fn card_update(mut cmd : Commands,
                   time : Res<Time>,
                   props : Res<Props>,
                   keyboard : Res<Input<KeyCode>>,
                   mut player : Query<(Entity, &mut Transform), With<Player>>,
                   mut cards : Query<(&mut DialogueCard<'static>, &mut Transform), Without<Player>>,
                   mut images: ResMut<Assets<Image>>,
                   mut materials: ResMut<Assets<StandardMaterial>>,
                   mut render_layer : Local<[bool;4]>) {
    // Obtain the player transformation
    if let Ok((player_entity, mut player_trans)) = player.get_single_mut() {
        //TODO: Delete this
        *player_trans = player_trans.looking_at(player_trans.translation + Vec3::new(time.elapsed_seconds().sin() * 0.02, -0.2, -1.), Vec3::Y);

        // TODO: DELETE Press x to spawn a card
        if keyboard.just_pressed(KeyCode::X) {
            create_card("x", player_entity, &mut cmd, &props, &mut images, &mut materials);
        }

        // Update the cards
        let n = cards.iter().count();
        for (i, (mut card, mut trans)) in cards.iter_mut().enumerate() {
            match card.status {
                CardStatus::Empty => {
                    // Get first available render layer
                    let layer = render_layer.iter().position(|&x| !x);
                    if let Some(layer) = layer {
                        render_layer[layer] = true;
                        card.render(layer as u8 + 1, &mut cmd).unwrap();
                    }
                    card.previous_trans = *trans;
                    card.lerp_time = 0.;
                },
                CardStatus::Rendered => {
                    if card.ready_counter > 0 {
                        card.ready_counter -= 1;
                        continue;
                    }
                    render_layer[card.render_layer.unwrap() as usize - 1] = false; 
                    card.clean(&mut cmd).unwrap();
                },
                CardStatus::Done => ()
            }

            let offset = i as f32 - (n as f32 - 1.) / 2.;
            card.target_trans.translation = Vec3::new(
                offset * CARD_PADDING.min(0.25 * 2.0 / n as f32),
                -0.35 + (i as f32 / (n-1) as f32 * std::f32::consts::PI).sin() * 0.02,
                -1. + i as f32 * 0.02 / n as f32
            );
            card.target_trans.rotation = Quat::from_rotation_z(offset * -0.3 / n as f32)
                .mul_quat(Quat::from_rotation_y(-offset * 0.05 / n as f32));

            // Check if rotations are equal
            if (trans.translation - card.target_trans.translation).length() < 0.01 && trans.rotation.dot(card.target_trans.rotation) > 0.99 {
                *trans = card.target_trans;
                card.previous_trans = card.target_trans;
                card.lerp_time = 0.;
            }
            if card.previous_trans != card.target_trans {
                card.lerp_time += time.delta_seconds();
                card.lerp_time = card.lerp_time.min(CARD_LERP_TIME);
                trans.translation = card.previous_trans.translation.lerp(card.target_trans.translation, smoothstep(card.lerp_time, 0., CARD_LERP_TIME));
                trans.rotation = card.previous_trans.rotation.lerp(card.target_trans.rotation, smoothstep(card.lerp_time, 0., CARD_LERP_TIME));
            }
        }
    }
}

// Pick cards using a mouse raycaster
pub fn pick_card_update(cam : Query<(&Camera, &GlobalTransform), With<Camera3d>>,
                        mut cards : Query<&mut DialogueCard<'static>>,
                        window: Query<&Window>) {
    let (cam, cam_trans) = cam.single();
    let Some(mouse_pos) = window.single().cursor_position() else { return; };
    let Some(ray) = cam.viewport_to_world(cam_trans, mouse_pos) else { return; };

    for mut card in cards.iter_mut() {
            
    }
}
