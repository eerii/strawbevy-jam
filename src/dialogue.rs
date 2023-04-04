// Dialogue system using yarn spinner

use bevy::{
    prelude::*,
    asset::{AssetLoader, LoadContext, LoadedAsset},
    utils::BoxedFuture,
    reflect::TypeUuid
};
use yarn_spinner::{LineHandler, YarnProgram, YarnRunner, YarnStorage};
pub use yarn_spinner::ExecutionOutput;

// ---
// Plugin

pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<DialogueRunner>()
           .init_asset_loader::<DialogueProgramAssetLoader>()
           .add_asset::<DialogueLines>()
           .init_asset_loader::<DialogueLinesAssetLoader>()
           .insert_resource(DialogueManager::new());
    }
}

// ---
// Resources

// Dialogue
#[derive(Resource, Default)]
pub struct DialogueManager {
    pub storage : YarnStorage,
    pub runner : Option<Handle<DialogueRunner>>,
    pub lines : Option<Handle<DialogueLines>>,
    pub waiting_continue : bool,
    pub waiting_response : bool
}

impl DialogueManager {
    pub fn new() -> DialogueManager {
        DialogueManager {
            storage : YarnStorage::new(),
            waiting_continue : false,
            waiting_response : false,
            ..default()
        }
    }

    pub fn load(&mut self, name : &str, assets : &Res<AssetServer>) {
        self.runner = Some(assets.load(format!("dialogue/build/{}.yarnc", name)));
        self.lines = Some(assets.load(format!("dialogue/build/{}.yarnl", name)));
    }
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
// Functions

// Return references to the runner and lines assets if they are loaded
pub fn get_dialogue_components<'a, 'b>(yarn : &'_ ResMut<DialogueManager>,
                                       asset_runner : &'a mut ResMut<Assets<DialogueRunner>>,
                                       asset_lines : &'b ResMut<Assets<DialogueLines>>) -> Option<(&'a mut YarnRunner, &'b LineHandler)> {
    let runner = yarn.runner.as_ref().expect("you need to load a dialogue with the dialogue manager");
    let lines = yarn.lines.as_ref().expect("you need to load a dialogue with the dialogue manager");
    
    let runner = asset_runner.get_mut(&runner);
    let lines = asset_lines.get(&lines);
    if runner.is_none() || lines.is_none() { return None; }

    let DialogueRunner(ref mut runner) = runner.unwrap();
    let DialogueLines(lines) = lines.unwrap();
    Some((runner, lines))
}
