#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;
use yarn_spinner::{ExecutionOutput, LineHandler, YarnProgram, YarnRunner, YarnStorage};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (init, dialogue))
        .run();
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: asset_server.load("icon.png"),
        ..Default::default()
    });
}

fn dialogue() {
    let mut storage = YarnStorage::new();
    let mut runner = YarnRunner::new(YarnProgram::new(include_bytes!("../assets/dialogue/build/test.yarnc")).unwrap());
    let lines = LineHandler::new(include_str!("../assets/dialogue/build/test-Lines.csv"));
    runner.set_node("Start").unwrap();

    while let Some(out) = runner.execute(&mut storage).unwrap() {
        match out {
            ExecutionOutput::Line(line) => {
                let text = lines.line(&line).unwrap();
                println!("{}", text);
                //std::io::stdin().read_line(&mut String::new()).unwrap();
            },
            ExecutionOutput::Options(opts) => {
                for opt in opts {
                    let v = lines.line(opt.line()).unwrap();
                    let v = match opt.condition_passed() {
                        Some(true) | None => v,
                        Some(false) => { format!("{} (DISABLED)", v) }
                    };
                    println!("{}", v);
                }
                let mut selection = String::new();
                std::io::stdin().read_line(&mut selection).unwrap();
                let opt = selection.trim().parse().unwrap();
                runner.select_option(opt).unwrap();
            },
            ExecutionOutput::Command(cmd) => {
                println!("todo: {:?}", cmd);
            },
            ExecutionOutput::Function(function) => {
                println!("func: {:?}", function);
                let output = yarn_spinner::handle_default_functions(&function).unwrap().unwrap();
                runner.return_function(output).unwrap();
            }
        }
    }
}
