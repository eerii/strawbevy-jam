#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::iter::zip;
use bevy::{
    prelude::*,
    asset::{Assets, AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
    reflect::TypeUuid
};
use yarn_spinner::{ExecutionOutput, LineHandler, YarnProgram, YarnRunner, YarnStorage};

// ---
// App

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_asset::<DialogueRunner>().init_asset_loader::<DialogueProgramAssetLoader>()
        .add_asset::<DialogueLines>().init_asset_loader::<DialogueLinesAssetLoader>()
        .add_systems(Startup, (init, dialogue_init))
        .add_systems(Update, dialogue_update)
        .run();
}

// ---
// Components

#[derive(Component)]
struct Dialogue {}

#[derive(Component)]
struct DialogueOption {}

// ---
// Resources

// Dialogue
#[derive(Resource)]
struct DialogueManager {
    storage : YarnStorage,
    runner : Handle<DialogueRunner>,
    lines : Handle<DialogueLines>,
    waiting_continue : bool,
    waiting_input : usize
}

// ---
// Assets

// Yarn file assets
#[derive(TypeUuid)]
#[uuid = "5de9f240-f252-428b-9f95-f58c38576db5"]
struct DialogueRunner(YarnRunner);

#[derive(Default)]
struct DialogueProgramAssetLoader;

impl AssetLoader for DialogueProgramAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let program = YarnProgram::new(bytes);
            let mut runner = YarnRunner::new(program.unwrap());
            runner.set_node("Start").unwrap();
            load_context.set_default_asset(LoadedAsset::new(DialogueRunner(runner)));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["yarnc"] }
}

#[derive(TypeUuid)]
#[uuid = "6b8867a2-4d12-40f9-a0e6-9ea20b207518"]
struct DialogueLines(LineHandler);

#[derive(Default)]
struct DialogueLinesAssetLoader;

impl AssetLoader for DialogueLinesAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let lines = LineHandler::new(std::str::from_utf8(bytes).unwrap());
            load_context.set_default_asset(LoadedAsset::new(DialogueLines(lines)));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["csv"] }
}

// ---
// Startup systems

// General initialization
fn init(mut cmd : Commands, assets : Res<AssetServer>) {
    // Camera
    cmd.spawn(Camera2dBundle::default());
    
    // Dialogue box
    let text_style = TextStyle {
        font : assets.load("fonts/dogicabold.ttf"),
        font_size : 24.0,
        color : Color::WHITE,
    };
    cmd.spawn((
        Text2dBundle {
            text : Text::from_section("", text_style.clone()),
            transform : Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Dialogue {}
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

// Yarn dialogue initialization
fn dialogue_init(mut cmd : Commands, assets : Res<AssetServer>) {
    cmd.insert_resource(DialogueManager {
        storage : YarnStorage::new(),
        runner : assets.load("dialogue/build/test.yarnc"),
        lines : assets.load("dialogue/build/test-Lines.csv"),
        waiting_continue : false,
        waiting_input : 0
    });
}

// ---
// Update systems

const OPTION_KEYS : &'static [KeyCode] = &[KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];

fn dialogue_update(keyboard : Res<Input<KeyCode>>, 
                   mut yarn : ResMut<DialogueManager>,
                   mut asset_runner : ResMut<Assets<DialogueRunner>>,
                   asset_lines : ResMut<Assets<DialogueLines>>,
                   mut dialogue_box : Query<&mut Text, With<Dialogue>>,
                   mut dialogue_options : Query<&mut Text, (With<DialogueOption>, Without<Dialogue>)>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let runner = asset_runner.get_mut(&yarn.runner);
    let lines = asset_lines.get(&yarn.lines);
    if runner.is_none() || lines.is_none() {
        return ();
    }
    let DialogueRunner(ref mut runner) = runner.unwrap();
    let DialogueLines(lines) = lines.unwrap();

    // If there are options, check if the user presses the relevant key
    if yarn.waiting_input > 0 {
        for i in 0..yarn.waiting_input {
            if keyboard.just_pressed(OPTION_KEYS[i]) {
                runner.select_option(i).unwrap();
                yarn.waiting_input = 0;
                for mut d in dialogue_options.iter_mut() {
                    d.sections[0].value = "".to_string();
                }
                break;
            }
        }
    }

    // Check if the dialogue is paused and if the user is continuing
    if keyboard.just_pressed(KeyCode::Space) {
        yarn.waiting_continue = false;
    }
    if yarn.waiting_continue || yarn.waiting_input > 0 {
        return ();
    }

    // Update the dialogue with the options
    if let Ok(Some(dialogue)) = runner.execute(&mut yarn.storage) {
        match dialogue {
            ExecutionOutput::Line(line) => {
                let new_line = lines.line(&line).unwrap();
                dialogue_box.single_mut().sections[0].value = new_line;
                yarn.waiting_continue = true;
            },
            ExecutionOutput::Options(opts) => {
                if opts.len() > 3 { todo!() }
                for (i, (mut d, v)) in zip(dialogue_options.iter_mut(), opts.iter()).enumerate() {
                    let opt = lines.line(v.line()).unwrap();
                    let opt = match v.condition_passed() {
                        Some(true) | None => format!("{} {opt}", i+1),
                        Some(false) => format!("{} {opt} (NO)", i+1)
                    };
                    d.sections[0].value = opt;
                }
                yarn.waiting_input = opts.len();
            },
            ExecutionOutput::Command(cmd) => {
                println!("todo: {:?}", cmd);
            },
            ExecutionOutput::Function(function) => {
                println!("func: {:?}", function);
                let output = yarn_spinner::handle_default_functions(&function);
                println!("output: {:?}", output);
                runner.return_function(output.unwrap().unwrap()).unwrap();
            }
        }
    }
}
