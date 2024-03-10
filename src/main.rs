mod decrypt;
mod encrypt;
mod shared;
use anyhow::{Result, *};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::Parser;
use decrypt::decrypt;
use encrypt::encrypt;
use reqwest::{blocking::Client, Url};
use std::{
    io::{self, Write},
    sync::{Arc, Condvar, Mutex},
};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    url: Url,
    #[arg(short, long, conflicts_with = "encrypt")]
    decrypt: Option<String>,
    #[arg(short, long, conflicts_with = "decrypt")]
    encrypt: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(ciphertext) = args.decrypt {
        decrypt(args.url.to_string(), &ciphertext)?;
        return Ok(());
    }

    if let Some(plaintext) = args.encrypt {
        encrypt(args.url.to_string(), &plaintext)?;
        return Ok(());
    }

    Ok(())
}
