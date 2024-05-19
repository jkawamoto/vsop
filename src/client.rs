// client.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdin};
use std::process::Command;

use anyhow::Result;
use clap::{crate_name, Parser};
use spinners::{Spinner, Spinners};
use tempfile::NamedTempFile;
use tokio::net::UnixStream;
use tonic::Request;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

use translator::Request as TranslationRequest;
use translator::translator_client::TranslatorClient;

use crate::socket::socket_filename;

#[allow(dead_code)]
mod socket;

const APP_NAME: &str = crate_name!();

pub mod translator {
    tonic::include_proto!("translator");
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Read source text from the specified file.
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    source_file: Option<String>,
    /// Read source text from standard input (stdin).
    #[arg(short, long)]
    stdin: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let r: Box<dyn Read> = if let Some(source_file) = args.source_file {
        Box::new(File::open(source_file)?)
    } else if args.stdin {
        Box::new(stdin())
    } else {
        let file = NamedTempFile::new()?;
        let _ = Command::new("nano").arg(file.path()).spawn()?.wait()?;
        Box::new(file)
    };
    let prompts = prepare_prompts(BufReader::new(r))?;

    let socket_file = socket_filename(APP_NAME)?;
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            // Connect to a Uds socket
            UnixStream::connect(socket_file.clone())
        }))
        .await?;

    let mut client = TranslatorClient::new(channel);
    let req = Request::new(TranslationRequest { source: prompts });
    let mut sp = Spinner::new(Spinners::Dots, "Translating...".to_string());
    let res = client.translate(req).await?;
    sp.stop_with_newline();
    for r in res.get_ref().result.iter() {
        println!("{}", r.trim());
    }

    Ok(())
}

fn prepare_prompts<R: BufRead>(r: R) -> Result<Vec<String>> {
    fn split_line(line:String)->Vec<String>{
        line.split_inclusive(".")
            .map(|s| String::from(s.trim()))
            .collect::<Vec<String>>()
    }

    let mut res = vec![];
    let mut line = String::new();
    for s in r.lines() {
        line.push_str(s?.trim());
        if line.ends_with(".") {
            res.append(&mut split_line(line));
            line = String::new();
        }else if line.is_empty(){
            res.push(String::new());
        } else {
            line.push_str(" ")
        }
    }
    if !line.is_empty() {
        res.append(&mut split_line(line));
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::prepare_prompts;

    #[test]
    fn test_prepare_prompts() {
        assert_eq!(
            prepare_prompts(Cursor::new("This is a sample text.")).unwrap(),
            vec!["This is a sample text.".to_string()]
        );
        assert_eq!(
            prepare_prompts(Cursor::new("This is\n a sample text.")).unwrap(),
            vec!["This is a sample text.".to_string()]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line. And this is the second line."
            ))
            .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And this is the second line.".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line.\nAnd this is the second line."
            ))
                .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And this is the second line.".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line. And\nthis is the second line."
            ))
                .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And this is the second line.".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line. And the second line misses the period"
            ))
                .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And the second line misses the period".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line.\nAnd the second line misses the period"
            ))
                .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And the second line misses the period".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first line. And\nthe second line misses the period"
            ))
                .unwrap(),
            vec![
                "This is the first line.".to_string(),
                "And the second line misses the period".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first paragraph.\n\nThis is the second paragraph."
            ))
                .unwrap(),
            vec![
                "This is the first paragraph.".to_string(),
                "".to_string(),
                "This is the second paragraph.".to_string(),
            ]
        );
        assert_eq!(
            prepare_prompts(Cursor::new(
                "This is the first\nparagraph.\n\nThis is the second paragraph."
            ))
                .unwrap(),
            vec![
                "This is the first paragraph.".to_string(),
                "".to_string(),
                "This is the second paragraph.".to_string(),
            ]
        );
        // assert_eq!(
        //     prepare_prompts(Cursor::new(
        //         "This is the first paragraph without the period\n\nThis is the second paragraph."
        //     ))
        //         .unwrap(),
        //     vec![
        //         "This is the first paragraph without the period".to_string(),
        //         "".to_string(),
        //         "This is the second paragraph.".to_string(),
        //     ]
        // );
    }
}
