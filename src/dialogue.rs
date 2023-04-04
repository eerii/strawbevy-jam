// Dialogue system using yarn spinner

use std::iter::zip;
use bevy::{
    prelude::*,
    asset::{Assets, AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
    reflect::TypeUuid
};
use yarn_spinner::{ExecutionOutput, LineHandler, YarnProgram, YarnRunner, YarnStorage};

const OPTION_KEYS : &[KeyCode] = &[KeyCode::Key1, KeyCode::Key2, KeyCode::Key3];

// ---
// Plugin

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<DialogueRunner>()
           .init_asset_loader::<DialogueProgramAssetLoader>()
           .add_asset::<DialogueLines>()
           .init_asset_loader::<DialogueLinesAssetLoader>();
    }
}

// ---
// Resources

// Dialogue
#[derive(Resource)]
pub struct DialogueManager {
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
pub struct DialogueRunner(pub YarnRunner);

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
pub struct DialogueLines(pub LineHandler);

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
    fn extensions(&self) -> &[&str] { &["yarnl"] }
}

// ---
// Components

#[derive(Component)]
pub struct DialogueBox {}

#[derive(Component)]
pub struct DialogueOption {}

// ---
// Systems

// Yarn dialogue initialization
pub fn dialogue_init(mut cmd : Commands, assets : Res<AssetServer>) {
    cmd.insert_resource(DialogueManager {
        storage : YarnStorage::new(),
        runner : assets.load("dialogue/build/test.yarnc"),
        lines : assets.load("dialogue/build/test.yarnl"),
        waiting_continue : false,
        waiting_input : 0
    });
}

pub fn dialogue_update(keyboard : Res<Input<KeyCode>>, 
                       mut yarn : ResMut<DialogueManager>,
                       mut asset_runner : ResMut<Assets<DialogueRunner>>,
                       asset_lines : ResMut<Assets<DialogueLines>>,
                       mut dialogue_box : Query<&mut Text, With<DialogueBox>>,
                       mut dialogue_options : Query<&mut Text, (With<DialogueOption>, Without<DialogueBox>)>) {
    // Get the assets for the dialogue manager and check that they are loaded
    let runner = asset_runner.get_mut(&yarn.runner);
    let lines = asset_lines.get(&yarn.lines);
    if runner.is_none() || lines.is_none() {
        return;
    }
    let DialogueRunner(ref mut runner) = runner.unwrap();
    let DialogueLines(lines) = lines.unwrap();

    // If there are options, check if the user presses the relevant key
    if yarn.waiting_input > 0 {
        for (i, key) in OPTION_KEYS.iter().enumerate().take(yarn.waiting_input) {
            if keyboard.just_pressed(*key) {
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
                let output = yarn_spinner::handle_default_functions(&function);
                runner.return_function(output.unwrap().unwrap()).unwrap();
            }
        }
    }
}
