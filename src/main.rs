#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use std::io::BufRead;

use bevy::prelude::*;
use yarn_spool::{read_string_table_file, expand_substitutions, Dialogue, DialogueEvent, Program as DialogueProgram};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, init)
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
    let dialogue_program = DialogueProgram::from_file("assets/dialogue/build/test.yarnc");
    let dialogue_lines = read_string_table_file("assets/dialogue/build/test-Lines.csv");

    let mut dialogue = Dialogue::new();
    dialogue.add_program(&dialogue_program);

    let mut input = String::new();

    while let Some(e) = dialogue.advance() {
        match e {
            DialogueEvent::Line => {
                let line = dialogue.current_line();
                let raw_text = &dialogue_lines[&line.id].text;
                let text = expand_substitutions(raw_text, &line.substitutions);
                println!("{}", text);
            },
            DialogueEvent::Command => {
                println!("<<{}>>", dialogue.current_command());
            },
            DialogueEvent::Options => {
                for opt in dialogue.current_options() {
                    let raw_text = &dialogue_lines[&opt.line.id].text;
                    let text = expand_substitutions(raw_text, &opt.line.substitutions);
                    println!("{}) {}", opt.index, text);
                }

                input.clear();
                std::io::stdin().lock().read_line(&mut input).expect("can't read input");
                let option = input.trim().parse().expect("can't parse input");
                dialogue.set_selected_option(option);
            }
        }
    }
}
