// main.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::io::{BufRead, BufReader};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use ct2rs::{TranslationOptions, Translator};
use directories::ProjectDirs;
use tempfile::NamedTempFile;

const APP_NAME: &str = "vsop";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the model to be used.
    #[arg(short, long, value_name = "NAME", default_value = "fugumt-en-ja")]
    model: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let dirs = ProjectDirs::from("", "", APP_NAME)
        .ok_or_else(|| anyhow!("failed to find home directory"))?;

    let model_path = dirs.cache_dir().join(args.model);
    let t = Translator::new(&model_path, &Default::default()).with_context(|| {
        format!(
            "failed to initialize a translator from {}",
            model_path.display()
        )
    })?;
    let opts = TranslationOptions {
        beam_size: 12,
        ..Default::default()
    };

    let file = NamedTempFile::new()?;
    let _ = Command::new("nano").arg(file.path()).spawn()?.wait()?;

    let mut prompts: Vec<String> = vec![];
    let mut prompt = String::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.len() == 0 {
            prompts.push(prompt);
            prompt = String::new();
        } else {
            prompt.push(' ');
            prompt.push_str(&line);
        }
    }

    let res = t.translate_batch(&prompts, &opts, None)?;
    for (r, _) in res {
        println!("{}", r.replace("。", "。\n"));
    }

    Ok(())
}
