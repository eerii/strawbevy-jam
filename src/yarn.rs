// Yarn spinner plugin for bevy, interface for the yarn_spinner crate

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

pub struct YarnPlugin;

impl Plugin for YarnPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<YarnRunnerAsset>()
           .init_asset_loader::<YarnRunnerAssetLoader>()
           .add_asset::<YarnLinesAsset>()
           .init_asset_loader::<YarnLinesAssetLoader>()
           .insert_resource(YarnManager::new());
    }
}

// ---
// Resources

// Dialogue
#[derive(Resource, Default)]
pub struct YarnManager {
    pub storage : YarnStorage,
    pub runner : Option<Handle<YarnRunnerAsset>>,
    pub lines : Option<Handle<YarnLinesAsset>>,
    pub waiting_continue : bool,
    pub waiting_response : bool
}

impl YarnManager {
    pub fn new() -> YarnManager {
        YarnManager {
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
pub struct YarnRunnerAsset(pub YarnRunner);

#[derive(Default)]
struct YarnRunnerAssetLoader;

impl AssetLoader for YarnRunnerAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let program = YarnProgram::new(bytes);
            let mut runner = YarnRunner::new(program.unwrap());
            runner.set_node("Start").unwrap();
            load_context.set_default_asset(LoadedAsset::new(YarnRunnerAsset(runner)));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["yarnc"] }
}

#[derive(TypeUuid)]
#[uuid = "6b8867a2-4d12-40f9-a0e6-9ea20b207518"]
pub struct YarnLinesAsset(pub LineHandler);

#[derive(Default)]
struct YarnLinesAssetLoader;

impl AssetLoader for YarnLinesAssetLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let lines = LineHandler::new(std::str::from_utf8(bytes).unwrap());
            load_context.set_default_asset(LoadedAsset::new(YarnLinesAsset(lines)));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] { &["yarnl"] }
}

// ---
// Functions

// Return references to the runner and lines assets if they are loaded
pub fn get_yarn_components<'a, 'b>(yarn : &'_ ResMut<YarnManager>,
                                   asset_runner : &'a mut ResMut<Assets<YarnRunnerAsset>>,
                                   asset_lines : &'b Res<Assets<YarnLinesAsset>>) -> Option<(&'a mut YarnRunner, &'b LineHandler)> {
    let runner = yarn.runner.as_ref().expect("you need to load a dialogue with the dialogue manager");
    let lines = yarn.lines.as_ref().expect("you need to load a dialogue with the dialogue manager");
    
    let runner = asset_runner.get_mut(runner);
    let lines = asset_lines.get(lines);
    if runner.is_none() || lines.is_none() { return None; }

    let YarnRunnerAsset(ref mut runner) = runner.unwrap();
    let YarnLinesAsset(lines) = lines.unwrap();
    Some((runner, lines))
}
