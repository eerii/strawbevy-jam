// Dialogue system using the yarn spinner plugin for bevy

use super::{Player, Props, AssetsLoading, StoryState, PersistentStorage, smoothstep, yarn::*, NUM_ENDINGS};
use std::{collections::{HashMap, hash_map::DefaultHasher}, cmp::Ordering, hash::{Hash, Hasher}, fmt::{Formatter, Debug}};
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

const SPEAKER_REMIE : Color = Color::rgb(0.6, 1.0, 0.7);
const SPEAKER_PLAYER : Color = Color::rgb(0.7, 0.6, 1.0);
const SPEAKER_WAITER : Color = Color::rgb(1.0, 0.7, 0.6);

// ---
// Resources

pub enum CardStatus {
    New(Option<usize>),
    Card(Entity, Option<usize>),
    Played,
}

impl Default for CardStatus {
    fn default() -> Self {
        CardStatus::New(None)
    }
}

pub enum WordType {
    Regular(String),
    Varying(String),
    PreviouslySelected(String),
}

impl Default for WordType {
    fn default() -> Self {
        WordType::Regular(String::new())
    }
}

impl Debug for WordType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WordType::Regular(s) => write!(f, "Regular({})", s),
            WordType::Varying(s) => write!(f, "Varying({})", s),
            WordType::PreviouslySelected(s) => write!(f, "PreviouslySelected({})", s),
        }
    }
}

#[derive(Resource, Default)]
pub struct DialogueState {
    pub selected_card : Option<Entity>,
    pub previous_card : Option<Entity>,
    pub cards : HashMap<String, (CardStatus, Vec<WordType>)>,
    pub important_decision : [Option<String> ; 2],
    pub important_id : [Option<Entity>; 2]
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

        // Create image text
        self.text_renderer = Some(cmd.spawn((
            Text2dBundle {
                text : Text::from_section(&self.id, self.style.clone()).with_alignment(TextAlignment::Center),
                text_2d_bounds : Text2dBounds{ size : Vec2::new(CARD_TEX_SIZE.width as f32 - 48., CARD_TEX_SIZE.height as f32 - 32.) },
                transform : Transform::from_xyz(0.5, 64., 0.1),
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

#[derive(Component)]
pub struct Drinks;

// ---
// Startup systems

// Resource initalization
pub fn res_init(mut cmd : Commands,
                assets : Res<AssetServer>,
                mut loading : ResMut<AssetsLoading>,
                mut fonts : ResMut<Assets<Font>>,
                mut meshes: ResMut<Assets<Mesh>>,
                mut yarn : ResMut<YarnManager>) {
    // Create plane mesh
    let card_mesh = meshes.add(Mesh::from(shape::Quad::new(CARD_MESH_SIZE)));
    let box_mesh = meshes.add(Mesh::from(shape::Quad::new(DIALOGUE_MESH_SIZE)));

    // Fonts (Bevy and rusttype)
    let font_data = include_bytes!("../assets/fonts/ponderosa.ttf");
    let font = fonts.add(Font::try_from_bytes(font_data.to_vec()).expect("Failed to load font"));

    // Dialogue text styles
    let mut box_style = HashMap::new();
    box_style.entry("regular").or_insert(TextStyle {
        font : font.clone(),
        font_size : DIALOGUE_FONT_SIZE,
        color : Color::WHITE,
    });
    box_style.entry("Remie").or_insert(TextStyle {
        font : font.clone(),
        font_size : DIALOGUE_FONT_SIZE,
        color : SPEAKER_REMIE,
    });
    box_style.entry("Player").or_insert(TextStyle {
        font : font.clone(),
        font_size : DIALOGUE_FONT_SIZE,
        color : SPEAKER_PLAYER,
    });
    box_style.entry("Waiter").or_insert(TextStyle {
        font : font.clone(),
        font_size : DIALOGUE_FONT_SIZE,
        color : SPEAKER_WAITER,
    });
    
    let mut card_style = HashMap::new();
    card_style.entry("regular").or_insert(TextStyle {
        font : font.clone(),
        font_size : CARD_FONT_SIZE,
        color : Color::BLACK,
    });
    card_style.entry("varying").or_insert(TextStyle {
        font : font.clone(),
        font_size : CARD_FONT_SIZE,
        color : Color::rgb(0.3, 0.3, 0.7),
    });
    card_style.entry("previously_selected").or_insert(TextStyle {
        font : font.clone(),
        font_size : CARD_FONT_SIZE,
        color : Color::rgb(0.05, 0.25, 0.05),
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
    loading.0.push(box_background.clone_untyped());
    loading.0.push(card_background.clone_untyped());

    // Drink images
    let drink_textures = [
        assets.load("textures/orange_juice.png"),
        assets.load("textures/beer.png"),
        assets.load("textures/water.png")
    ];
    for drink in drink_textures.iter() {
        loading.0.push(drink.clone_untyped());
    }

    // Save state
    cmd.insert_resource(Props { box_mesh, box_style, box_background,
                                card_mesh, card_style, card_texture_descriptor, card_background,
                                drink_textures, font });

    cmd.insert_resource(DialogueState::default());

    // Load dialogue
    yarn.load("dialogue", &assets);
}

// Dialogue box initialization
pub fn box_init(mut cmd : Commands,
                props : Res<Props>,
                mut images : ResMut<Assets<Image>>,
                mut materials : ResMut<Assets<StandardMaterial>>,
                player : Query<Entity, With<Player>>) {
    // Create dialogue image
    let mut image = Image { texture_descriptor : props.card_texture_descriptor.clone(), ..default() };
    image.resize(DIALOGUE_TEX_SIZE);
    let image_handle = images.add(image);

    // Dialogue text camera
    let box_pass_layer = RenderLayers::layer(2);
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
            text : Text::from_sections([
                TextSection::new("", props.box_style["regular"].clone()),
                TextSection::new("", props.box_style["regular"].clone()),
            ]),
            text_2d_bounds : Text2dBounds{ size : Vec2::new(DIALOGUE_TEX_SIZE.width as f32 - 48., DIALOGUE_TEX_SIZE.height as f32 - 32.) },
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
    let dial_box = cmd.spawn(
        PbrBundle {
            mesh: props.box_mesh.clone(),
            material: materials.add(StandardMaterial {
                base_color_texture : Some(image_handle),
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 2.8, -10.0),
            ..default()
        },
    ).id();
    cmd.entity(player.single()).push_children(&[dial_box]);
}

// ---
// Update systems

fn hash_obj<T>(obj : T) -> u64 where T : Hash {
    let mut h = DefaultHasher::new();
    obj.hash(&mut h);
    h.finish()
}

// Handle the changes in dialogue updates
pub fn update(mut cmd : Commands,
              mut state : ResMut<DialogueState>,
              mut story : ResMut<StoryState>,
              keyboard : Res<Input<KeyCode>>,
              mouse : Res<Input<MouseButton>>,
              props : Res<Props>,
              time : Res<Time>,
              mut storage : ResMut<PersistentStorage>,
              asset_lines : Res<Assets<YarnLinesAsset>>,
              mut asset_runner : ResMut<Assets<YarnRunnerAsset>>,
              mut yarn : ResMut<YarnManager>,
              mut materials : ResMut<Assets<StandardMaterial>>,
              mut dialogue_box : Query<&mut Text, With<DialogueBox>>,
              cards : Query<(Entity, &DialogueCard)>,
              mut wait_timer : Local<(f32, f32)>,
              mut other_option : Local<usize>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let (runner, lines) = match get_yarn_components(&yarn, &mut asset_runner, &asset_lines) {
        None => return,
        Some(v) => v
    };

    if yarn.finished {
        if keyboard.just_pressed(KeyCode::Space) {
            story.the_end = true;
        }
        return;
    }

    // Advance the wait timer
    if wait_timer.0 <= wait_timer.1 {
        wait_timer.0 += time.delta_seconds();
        return;
    }

    // Select a card using your mouse
    if yarn.waiting_response && mouse.just_pressed(MouseButton::Left) && state.selected_card.is_some() {
        let id = state.selected_card.unwrap();

        let (_, card) = cards.get(id).expect("Error loading card with selected card id");

        cmd.entity(id).despawn();
        yarn.waiting_response = false;

        let state_card = &mut state.cards.get_mut(&card.id);
        if let Some((st, _)) = state_card {
            if let CardStatus::Card(_, opt) = st {
                let opt = if opt.is_some() {opt.unwrap()} else {*other_option};
                runner.select_option(opt).unwrap();
                println!("Selected option {} with card {}", opt, card.id);
                *st = CardStatus::Played;

                let question = story.current_question;
                let opts = story.selected_options.entry(question).or_insert(vec![]);
                opts.push(card.id.clone());
                if storage.0.set("selected_options", &story.selected_options).is_err() {
                    println!("Warning, problem saving selected options");
                };
            }
        }
        state.selected_card = None;
        state.previous_card = None;

        if yarn.important_decision {
            let card_a = &state.important_decision[0].clone().unwrap();
            let card_b = &state.important_decision[1].clone().unwrap();
            // TODO: Despawn the cards

            for (id, card) in cards.iter() {
                if card.id == *card_a || card.id == *card_b {
                    cmd.entity(id).despawn();
                }
            }

            let card_a = state.cards.get_mut(&card_a.clone());
            if let Some((st, _)) = card_a {
                if let CardStatus::Card(_, _) = st {
                    *st = CardStatus::Played; 
                }
            }

            let card_b = state.cards.get_mut(&card_b.clone());
            if let Some((st, _)) = card_b {
                if let CardStatus::Card(_, _) = st {
                    *st = CardStatus::Played; 
                }
            }

            yarn.important_decision = false;
        }
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
                let mut l = lines.line(&line).expect("Failed to parse yarn line");
                let is_question = l.contains("___");

                while let Some(start) = l.find('[') {
                    let end = l.find(']').expect("Missing ] in option");
                    l.replace_range(start..end + 1, "");
                }

                let l : Vec<&str> = l.split(':').collect();

                let (speaker, style) = if l.len() == 1 {
                    println!("Warning, line without speaker");
                    ("".to_string(), props.box_style["regular"].clone())
                } else if !["Remie", "Player", "Waiter"].contains(&l[0]) {
                    println!("Warning, the speaker name is misspelled {}", l[0]);
                    (l[0].to_string() + "\n", props.box_style["regular"].clone())
                } else {
                    (l[0].to_string() + "\n", props.box_style[l[0]].clone())
                };
                let line = if l.len() == 1 { l[0].to_string() } else { l[1..].join(":") };

                if is_question {
                    story.current_question = hash_obj(&line);
                }

                dialogue_box.single_mut().sections[0] = TextSection::new(speaker, style);
                dialogue_box.single_mut().sections[1].value = line;

                yarn.waiting_continue = !is_question;
            },
            ExecutionOutput::Options(opts) => {
                for (_, (t, _)) in state.cards.iter_mut() {
                    if let CardStatus::Card(_, opt) = t {
                        *opt = None;
                    }
                }
                let question = story.current_question;

                for (opt_num, opt) in opts.iter().enumerate() {
                    let line = lines.line(opt.line()).expect("Failed to parse yarn option");

                    for l in line.split('|') {
                        let l = l.trim();
                        if l == "other" {
                            *other_option = opt_num;
                            continue;
                        }

                        let key : Vec<&str> = l.split(' ')
                            .filter(|x| !x.contains('('))
                            .collect();
                        let key = key.join(" ");
                        let prev_sel = story.selected_options.entry(question).or_default().contains(&key);

                        if l.starts_with('!') {
                            // Just two cards, hide the others
                            if state.important_decision[0].is_none() {
                                state.important_decision[0] = Some(key.to_string());
                            } else {
                                state.important_decision[1] = Some(key.to_string());
                            }

                            yarn.important_decision = true;
                        } 

                        let mut words = vec![];
                        for w in l.split(' ') {
                            if w.contains('(') {
                                words.push(WordType::Varying(w.replace(['(', ')'], "") + " "));
                                continue;
                            }
                            if words.last().is_some() && !matches!(words.last().unwrap(), WordType::Varying(_)) {
                                match words.last_mut().unwrap() {
                                    WordType::Regular(s) => { s.push(' '); s.push_str(w); },
                                    WordType::PreviouslySelected(s) => { s.push(' '); s.push_str(w); },
                                    _ => ()
                                }
                            } else if prev_sel {
                                words.push(WordType::PreviouslySelected(w.to_string()));
                            } else {
                                words.push(WordType::Regular(w.to_string()));
                            }
                        }

                        let (t, w) = state.cards.entry(key.to_string()).or_default();                        
                        *w = words;
                        match t {
                            CardStatus::New(o) => *o = Some(opt_num),
                            CardStatus::Card(_, o) => *o = Some(opt_num),
                            _ => ()
                        }
                        println!("Option {} with key {}", opt_num, key);
                    }
                }

                yarn.waiting_response = true;
            },
            ExecutionOutput::Command(c) => {
                let c : Vec<&str> = c.split(' ').collect();
                match c[0] {
                    "discard" => {
                        state.cards.iter_mut().for_each(|(_, (card, _))| {
                            if let CardStatus::Card(id, _) = card {
                                cmd.entity(*id).despawn();
                            } 
                            *card = CardStatus::Played;
                        });
                        state.selected_card = None;
                        state.previous_card = None;

                        dialogue_box.single_mut().sections[0].value = "".to_string(); 
                        dialogue_box.single_mut().sections[1].value = "New act (cards are discarded)".to_string();

                        yarn.waiting_continue = true;
                    },
                    "marcoComes" => {
                        story.is_marco_here = true;
                    },
                    "marcoLeaves" => {
                        story.is_marco_here = false;
                        if c.len() == 2 && c[1] == "drinks" {
                            if !yarn.storage.contains_key("$drink") {
                                println!("Warning, no drinks selected");
                            } else {
                                let drink_tex = match yarn.storage.get("$drink").unwrap().to_string().as_str() {
                                    "orangejuice" => props.drink_textures[0].clone(),
                                    "beer" => props.drink_textures[1].clone(),
                                    "water" => props.drink_textures[2].clone(),
                                    "weird" => props.drink_textures[2].clone(),
                                    _ => { println!("Warning, unsupported drink"); props.drink_textures[2].clone() }
                                };

                                // Remie's drink
                                cmd.spawn((
                                    PbrBundle {
                                        mesh: props.card_mesh.clone(),
                                        material: materials.add(StandardMaterial {
                                            base_color_texture : Some(props.drink_textures[0].clone()),
                                            alpha_mode: AlphaMode::Mask(0.5),
                                            ..default()
                                        }),
                                        transform: Transform::from_xyz(-4.5, 4.0, 9.5)
                                            .with_rotation(Quat::from_rotation_y(-0.2))
                                            .with_scale(Vec3::splat(2.5)),
                                        ..default()
                                    },
                                    Drinks{}
                                ));

                                // Player's drink
                                cmd.spawn((
                                    PbrBundle {
                                        mesh: props.card_mesh.clone(),
                                        material: materials.add(StandardMaterial {
                                            base_color_texture : Some(drink_tex),
                                            alpha_mode: AlphaMode::Mask(0.5),
                                            ..default()
                                        }),
                                        transform: Transform::from_xyz(-4.0, 4.0, 10.7)
                                            .with_rotation(Quat::from_rotation_y(-0.4))
                                            .with_scale(Vec3::splat(2.5)),
                                        ..default()
                                    },
                                    Drinks{}
                                ));
                            }
                        }
                    },
                    "remieLeaves" => {
                        story.is_remie_here = false;
                    },
                    "wait" => {
                        *wait_timer = (0., if c.len() > 1 { c[1].parse::<f32>().expect("Error converting string") } else { 1. });
                        dialogue_box.single_mut().sections[0].value = "".to_string(); 
                        dialogue_box.single_mut().sections[1].value = "...".to_string();
                    },
                    "theEnd" => {
                        assert!(c.len() == 2, "Error parsing theEnd");
                        let num = c[1].parse::<usize>().expect("Error parsing ending number") - 1;
                        assert!(num < NUM_ENDINGS, "There are too many endings!");
                        story.endings[num] = true;
                        if storage.0.set("unlocked_endings", &story.endings).is_err() {
                            println!("Warning, problem saving unlocked endings");
                        }
                        dialogue_box.single_mut().sections[0].value = "".to_string(); 
                        dialogue_box.single_mut().sections[1].value = "the end... or is it".to_string();
                        yarn.finished = true;
                    },
                    _ => println!("TODO: Command not implemented {}", c[0])
                }
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
                           mut state : ResMut<DialogueState>,
                           mut images : ResMut<Assets<Image>>,
                           mut materials: ResMut<Assets<StandardMaterial>>,
                           player : Query<Entity, With<Player>>) {
    for (word, (card, _)) in state.cards.iter_mut() {
        if let CardStatus::New(opt) = card {
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
            *card = CardStatus::Card(id, *opt);
        }
    }
}

// Update the cards with new words
pub fn card_words_update(state : ResMut<DialogueState>,
                         props : ResMut<Props>,
                         cards : Query<&DialogueCard>,
                         mut text : Query<&mut Text>) {
    for card in cards.iter() {
        let (st, words) = &state.cards.get(&card.id).expect("Error loading card with id");
        if let CardStatus::Played = st { continue; }

        if let Some(rend) = card.text_renderer {
            if let Ok(mut t) = text.get_mut(rend) {
                t.sections = words.iter().map(|x| match x {
                    WordType::Regular(w) => TextSection::new(w, props.card_style["regular"].clone()),
                    WordType::Varying(w) => TextSection::new(w, props.card_style["varying"].clone()),
                    WordType::PreviouslySelected(w) => TextSection::new(w, props.card_style["previously_selected"].clone()),
                }).collect();
            }
        }
    }
}

// Updates the cards position and attributes
pub fn card_update(mut cmd : Commands,
                   time : Res<Time>,
                   props : Res<Props>,
                   state : Res<DialogueState>,
                   yarn : Res<YarnManager>,
                   mut cards : Query<(Entity, &mut DialogueCard, &mut Transform), Without<Player>>,
                   mut render_layer : Local<u8>) {
    if *render_layer == 0 { *render_layer = 2 };

    let n = cards.iter().count();
    for (i, (e, mut card, mut trans)) in cards.iter_mut().enumerate() {
        if !card.has_renderer {
            *render_layer += 1;
            assert!(*render_layer < 32, "Can't have more than 32 render layers");
            card.render(*render_layer, &mut cmd, &props); 
            card.previous_trans = *trans;
            card.lerp_time = 0.;
            card.has_renderer = true;
        }

        if yarn.important_decision {
            if state.important_decision[0].is_some() && state.important_decision[0].clone().unwrap() == card.id {
                card.target_trans.translation = Vec3::new(-0.15, 0., -1.);
            } else if state.important_decision[1].is_some() && state.important_decision[1].clone().unwrap() == card.id {
                card.target_trans.translation = Vec3::new(0.15, 0., -1.); 
            } else {
                card.target_trans.translation = Vec3::new(0., -0.8, 0.);
            }
        } else {
            let offset = i as f32 - (n as f32 - 1.) / 2.;
            card.target_trans.translation = Vec3::new(
                offset * CARD_PADDING.min(0.28 * 2.0 / n as f32),
                -0.35 + (i as f32 / (if n > 1 {n-1} else {1}) as f32 * std::f32::consts::PI).sin() * 0.02,
                -1. + i as f32 * 0.02 / n as f32
            );
            card.target_trans.rotation = Quat::from_rotation_z(offset * -0.3 / n as f32)
                .mul_quat(Quat::from_rotation_y(-offset * 0.05 / n as f32));

            if !yarn.waiting_response {
                card.target_trans.translation += Vec3::new(0., -0.08, 0.);
            }

            if state.selected_card.is_some() && state.selected_card.unwrap() == e {
                if yarn.waiting_response {
                    card.target_trans.translation += Vec3::new(0., 0.1, 0.05);
                }
                if state.previous_card.is_none() || state.previous_card.unwrap() != e {
                    card.previous_trans = *trans;
                    card.lerp_time = 0.;
                }
            } else if state.previous_card.is_some() && state.previous_card.unwrap() == e {
                card.previous_trans = *trans;
                card.lerp_time = 0.;
            }
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
pub fn pick_card_update(mut state : ResMut<DialogueState>,
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