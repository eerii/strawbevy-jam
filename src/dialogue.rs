// Dialogue system using the yarn spinner plugin for bevy

use super::{
    Player,
    yarn::*
};
use std::{collections::HashMap, cmp::Ordering};
use bevy::{
    prelude::*,
    core_pipeline::clear_color::ClearColorConfig,
    text::Text2dBounds,
    render::{
        render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers, camera::RenderTarget
    }, 
};

// ---
// Constants

const CARD_PADDING : f32 = 0.18;
const CARD_TEX_SIZE : Extent3d = Extent3d { width: 256, height: 320, depth_or_array_layers: 1 };
const CARD_MESH_SIZE : Vec2 = Vec2::new(0.2, 0.25);
const CARD_LERP_TIME : f32 = 0.2;
const CARD_FONT_SIZE : f32 = 24.;

const DIALOGUE_TEX_SIZE : Extent3d = Extent3d { width: 768, height: 192, depth_or_array_layers: 1 };
const DIALOGUE_MESH_SIZE : Vec2 = Vec2::new(6.0, 1.5);
const DIALOGUE_FONT_SIZE : f32 = 24.;

// ---
// Resources

#[derive(Resource)]
pub struct Props {
    box_mesh : Handle<Mesh>,
    box_style : HashMap<&'static str, TextStyle>,
    box_background : Handle<Image>,
    card_mesh : Handle<Mesh>,
    card_style : HashMap<&'static str, TextStyle>,
    card_texture_descriptor : TextureDescriptor<'static>,
    card_background : Handle<Image>,
    rt_font : rusttype::Font<'static>,
}

enum CardState {
    New,
    Card(Entity),
    Played,
}

enum WordType {
    Regular(String),
    Varying(String),
}

#[derive(Resource, Default)]
pub struct State {
    selected_card : Option<Entity>,
    previous_card : Option<Entity>,
    cards : HashMap<String, (CardState, Vec<WordType>)>,
}

// ---
// Components

#[derive(Component)]
pub struct DialogueBox;

#[derive(Component, Default, Debug)]
pub struct DialogueCard {
    id : String,
    has_renderer : bool,
    image : Handle<Image>,
    style : TextStyle,
    render_layer : Option<u8>,
    previous_trans : Transform,
    target_trans : Transform,
    lerp_time : f32,
    text_renderer : Option<Entity>,
}

impl DialogueCard {
    fn new(id : String, image : Handle<Image>, style : TextStyle) -> DialogueCard {
        DialogueCard { id, image, style, ..default() }
    }

    fn render(&mut self, render_layer : u8, cmd : &mut Commands, props : &Res<Props>) {
        // Camera to render the 2d text onto the card image
        let text_pass_layer = RenderLayers::layer(render_layer);
        cmd.spawn((
            Camera2dBundle {
                camera: Camera {
                    order: -1,
                    target: RenderTarget::Image(self.image.clone()),
                    ..default()
                },
                ..default()
            },
            text_pass_layer
        ));

        // Use rusttype to get the text width
        // There is a weird bug where if the text wraps it is not centered
        // So we are using this to detect if it is long and to move it a few pixels
        let width = props.rt_font
            .layout(&self.id, rusttype::Scale::uniform(CARD_FONT_SIZE), rusttype::point(0.0, 0.0))
            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
            .last()
            .unwrap_or(0.0);

        // Create image text
        self.text_renderer = Some(cmd.spawn((
            Text2dBundle {
                text : Text::from_section(&self.id, self.style.clone()).with_alignment(TextAlignment::Center),
                text_2d_bounds : Text2dBounds{ size : Vec2::new(CARD_TEX_SIZE.width as f32 - 48., CARD_TEX_SIZE.height as f32 - 32.) },
                transform : Transform::from_xyz(if width > CARD_TEX_SIZE.width as f32 - 48. {6.5} else {0.5}, 64., 0.1),
                ..default()
            },
            text_pass_layer
        )).id());

        // Create card background sprite
        cmd.spawn((
            SpriteBundle {
                texture : props.card_background.clone(),
                transform : Transform::from_scale(Vec3::splat(8.)),
                ..default()
            },
            text_pass_layer
        ));

        self.render_layer = Some(render_layer);
    }
}

// ---
// Startup systems

// Resource initalization
pub fn res_init(mut cmd : Commands,
                assets : Res<AssetServer>,
                mut fonts : ResMut<Assets<Font>>,
                mut meshes: ResMut<Assets<Mesh>>,
                mut yarn : ResMut<YarnManager>) {
    // Create plane mesh
    let card_mesh = meshes.add(Mesh::from(shape::Quad::new(CARD_MESH_SIZE)));
    let box_mesh = meshes.add(Mesh::from(shape::Quad::new(DIALOGUE_MESH_SIZE)));

    // Fonts (Bevy and rusttype)
    let font_data = include_bytes!("../assets/fonts/ponderosa.ttf");
    let font = fonts.add(Font::try_from_bytes(font_data.to_vec()).expect("Failed to load font"));
    let rt_font = rusttype::Font::try_from_bytes(font_data).expect("Failed to load font");

    // Dialogue text styles
    let mut box_style = HashMap::new();
    box_style.entry("regular").or_insert(TextStyle {
        font : font.clone(),
        font_size : DIALOGUE_FONT_SIZE,
        color : Color::WHITE,
    });
    
    let mut card_style = HashMap::new();
    card_style.entry("regular").or_insert(TextStyle {
        font : font.clone(),
        font_size : CARD_FONT_SIZE,
        color : Color::BLACK,
    });
    card_style.entry("varying").or_insert(TextStyle {
        font,
        font_size : CARD_FONT_SIZE,
        color : Color::BLUE,
    });

    // Card texture properties
    let card_texture_descriptor = TextureDescriptor {
        label: None,
        size: CARD_TEX_SIZE,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUsages::TEXTURE_BINDING
             | TextureUsages::COPY_DST
             | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[TextureFormat::Rgba8Unorm],
    };

    // Background images
    let box_background = assets.load("textures/dialogue.png");
    let card_background = assets.load("textures/card.png");

    // Save state
    cmd.insert_resource(Props { box_mesh, box_style, box_background,
                                card_mesh, card_style, card_texture_descriptor, card_background, rt_font });

    cmd.insert_resource(State::default());

    // Load dialogue
    yarn.load("dialogue", &assets);
}

// Dialogue box initialization
pub fn box_init(mut cmd : Commands,
                props : Res<Props>,
                mut images : ResMut<Assets<Image>>,
                mut materials : ResMut<Assets<StandardMaterial>>) {
    // Create dialogue image
    let mut image = Image { texture_descriptor : props.card_texture_descriptor.clone(), ..default() };
    image.resize(DIALOGUE_TEX_SIZE);
    let image_handle = images.add(image);

    // Dialogue text camera
    let box_pass_layer = RenderLayers::layer(1);
    cmd.spawn((
        Camera2dBundle{
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None
            },
            camera: Camera {
                order: -2,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            ..default()
        },
        box_pass_layer
    )); 

    // Dialogue box
    cmd.spawn((
        Text2dBundle {
            text : Text::from_section("", props.box_style["regular"].clone()),
            transform : Transform::from_xyz(0.0, 0.0, 0.1),
            ..default()
        },
        DialogueBox{},
        box_pass_layer
    ));
    cmd.spawn((
        SpriteBundle {
            texture : props.box_background.clone(),
            transform : Transform::from_scale(Vec3::splat(8.)),
            ..default()
        },
        box_pass_layer
    ));

    // Actual mesh in the 3d camera
    cmd.spawn(
        PbrBundle {
            mesh: props.box_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color_texture : Some(image_handle),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }),
            transform: Transform::from_xyz(-5.4, 7.0, 4.5),
            ..default()
        },
    );
}

// ---
// Update systems

// Handle the changes in dialogue updates
pub fn update(mut cmd : Commands,
              mut state : ResMut<State>,
              keyboard : Res<Input<KeyCode>>,
              asset_lines : Res<Assets<YarnLinesAsset>>,
              mut asset_runner : ResMut<Assets<YarnRunnerAsset>>,
              mut yarn : ResMut<YarnManager>,
              mut dialogue_box : Query<&mut Text, With<DialogueBox>>,
              cards : Query<&DialogueCard>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let (runner, lines) = match get_yarn_components(&yarn, &mut asset_runner, &asset_lines) {
        None => return,
        Some(v) => v
    };

    // For now just use the first response when having options
    // This will be handled by selecting a card
    if yarn.waiting_response && keyboard.just_pressed(KeyCode::Z) && state.selected_card.is_some() {
        let id = state.selected_card.unwrap();

        let card = cards.get(id).expect("Error loading card with selected card id");
        runner.select_option(0).unwrap(); //TODO: Choose card

        cmd.entity(id).despawn();
        yarn.waiting_response = false;

        state.cards.get_mut(&card.id).unwrap().0 = CardState::Played;
        state.selected_card = None;
        state.previous_card = None;
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
                let new_line = lines.line(&line).expect("Failed to parse yarn line");
                dialogue_box.single_mut().sections[0].value = new_line;
                yarn.waiting_continue = true;
            },
            ExecutionOutput::Options(opts) => {
                for opt in opts {
                    let l = lines.line(opt.line()).expect("Failed to parse yarn option");

                    let key : Vec<&str> = l.split(" ")
                        .filter(|x| !x.contains("("))
                        .collect();
                    let key = key.join(" ");

                    let mut words = vec![];
                    for w in l.split(" ") {
                        if w.contains("(") {
                            words.push(WordType::Varying(w.replace("(", "").replace(")", "") + " "));
                            continue;
                        }
                        if words.last().is_some() && matches!(words.last().unwrap(), WordType::Regular(_)) {
                            if let WordType::Regular(s) = words.last_mut().unwrap() {
                                s.push_str(" ");
                                s.push_str(w);
                            }
                        } else {
                            words.push(WordType::Regular(w.to_string()));
                        }
                    }

                    let (_, w) = state.cards.entry(key.to_string()).or_insert((CardState::New, vec![]));
                    *w = words;
                }

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

// Create the cards requested
pub fn create_cards_update(mut cmd : Commands,
                           props : Res<Props>,
                           mut state : ResMut<State>,
                           mut images : ResMut<Assets<Image>>,
                           mut materials: ResMut<Assets<StandardMaterial>>,
                           player : Query<Entity, With<Player>>) {
    for (word, (card, _)) in state.cards.iter_mut().filter(|(_, (card, _))| matches!(card, CardState::New) ) {
        let mut image = Image { texture_descriptor : props.card_texture_descriptor.clone(), ..default() };
        image.resize(CARD_TEX_SIZE);
        let image_handle = images.add(image);

        let id = cmd.spawn((
            PbrBundle {
                mesh: props.card_mesh.clone(),
                material: materials.add(StandardMaterial {
                    base_color_texture : Some(image_handle.clone()),
                    ..default()
                }),
                transform: Transform::from_xyz(0., -1., -1.),
                ..default()
            },
            DialogueCard::new(word.to_string(), image_handle, props.card_style["regular"].clone()),
        )).id();

        cmd.entity(player.single()).push_children(&[id]); 
        *card = CardState::Card(id);
    }
}

// Update the cards with new words
pub fn card_words_update(state : ResMut<State>,
                         props : ResMut<Props>,
                         cards : Query<&DialogueCard>,
                         mut text : Query<&mut Text>) {
    for card in cards.iter() {
        let (st, words) = &state.cards.get(&card.id).expect("Error loading card with id");
        if let CardState::Played = st { continue; }

        if let Some(rend) = card.text_renderer {
            if let Ok(mut t) = text.get_mut(rend) {
                t.sections = words.iter().map(|x| match x {
                    WordType::Regular(w) => TextSection::new(w, props.card_style["regular"].clone()),
                    WordType::Varying(w) => TextSection::new(w, props.card_style["varying"].clone()),
                }).collect();
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
                   state : Res<State>,
                   mut cards : Query<(Entity, &mut DialogueCard, &mut Transform), Without<Player>>,
                   mut render_layer : Local<u8>) {
    if *render_layer == 0 { *render_layer = 1 };

    let n = cards.iter().count();
    for (i, (e, mut card, mut trans)) in cards.iter_mut().enumerate() {
        if !card.has_renderer {
            *render_layer += 1;
            card.render(*render_layer, &mut cmd, &props); 
            card.previous_trans = *trans;
            card.lerp_time = 0.;
            card.has_renderer = true;
        }

        let offset = i as f32 - (n as f32 - 1.) / 2.;
        card.target_trans.translation = Vec3::new(
            offset * CARD_PADDING.min(0.28 * 2.0 / n as f32),
            -0.35 + (i as f32 / (if n > 1 {n-1} else {1}) as f32 * std::f32::consts::PI).sin() * 0.02,
            -1. + i as f32 * 0.02 / n as f32
        );
        card.target_trans.rotation = Quat::from_rotation_z(offset * -0.3 / n as f32)
            .mul_quat(Quat::from_rotation_y(-offset * 0.05 / n as f32));

        if state.selected_card.is_some() && state.selected_card.unwrap() == e {
            card.target_trans.translation += Vec3::new(0., 0.1, 0.05);
            if state.previous_card.is_none() || state.previous_card.unwrap() != e {
                card.previous_trans = *trans;
                card.lerp_time = 0.;
            }
        } else if state.previous_card.is_some() && state.previous_card.unwrap() == e {
            card.previous_trans = *trans;
            card.lerp_time = 0.;
        }

        if (trans.translation - card.target_trans.translation).length() < 0.01 {
            *trans = card.target_trans;
            card.previous_trans = card.target_trans;
            card.lerp_time = 0.;
        }

        if card.target_trans != *trans {
            card.lerp_time += time.delta_seconds();
            card.lerp_time = card.lerp_time.min(CARD_LERP_TIME);
            trans.translation = card.previous_trans.translation.lerp(card.target_trans.translation, smoothstep(card.lerp_time, 0., CARD_LERP_TIME));
            trans.rotation = card.previous_trans.rotation.lerp(card.target_trans.rotation, smoothstep(card.lerp_time, 0., CARD_LERP_TIME));
        }
    } 
}

fn intersect(plane_center : Vec3, plane_normal : Vec3,
             view_pos : Vec3, view_dir : Vec3) -> Option<(f32, Vec3)> {
    if view_dir.dot(plane_normal) == 0. { 
        return None;
    }
    
    let d = (plane_center - view_pos).dot(plane_normal) / view_dir.dot(plane_normal);
    let p = view_pos + view_dir * d;
    Some((d, p))
}

// Pick cards using a mouse raycaster
pub fn pick_card_update(mut state : ResMut<State>,
                        cam : Query<(&Camera, &GlobalTransform), With<Camera3d>>,
                        cards : Query<(Entity, &GlobalTransform), With<DialogueCard>>,
                        window : Query<&Window>,
                        mut mouse_prev : Local<Vec2>) {
    // Get the mouse world position
    let Some(mouse_pos) = window.single().cursor_position() else { return; };
    *mouse_prev = mouse_pos;

    let (cam, cam_trans) = cam.single();
    let Some(ray) = cam.viewport_to_world(cam_trans, mouse_pos) else { return; };

    // Iterate through all the cards to find out which of them are being hovered
    let mut cards_hovered = HashMap::new();
    let card_bounds = Vec3::from((CARD_MESH_SIZE * 0.5, 0.));

    for (e, trans) in cards.iter() {
        let (_, r, t) = trans.to_scale_rotation_translation();
        let normal = r.mul_vec3(Vec3::Z);

        // Create intersection
        let Some((_, point)) = intersect(t, normal, ray.origin, ray.direction) else { continue; };

        // Check if the point is the the card mesh bounds
        let point = point - t;
        if point.x.abs() > card_bounds.x || point.y.abs() > card_bounds.y { continue; }

        // Add to a list of hovered cards
        let dist = point.x.abs() - card_bounds.x;
        cards_hovered.insert(e, dist);
    }

    // Find the closest hovered card (or the previous card)
    if let Some((&e, _)) = cards_hovered.iter().min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal)) {
        state.previous_card = state.selected_card;
        state.selected_card = Some(e);
    } else {
        state.previous_card = None;
        state.selected_card = None;
    }
}
