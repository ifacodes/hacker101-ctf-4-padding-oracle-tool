use std::{process::Output, thread::JoinHandle};

use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use reqwest::Client;

static CHUNK_SIZE: usize = 16;

trait ModBase64 {
    type Output;
    fn fix_base64(self) -> Self::Output;
    fn fuckup_base64(self) -> Self::Output;
}

fn decrypt() -> Result<()> {
    let ciphertext = "";
    let url = "";
    let client = Client::new();

    for (n, chunks) in BASE64_STANDARD
        .decode(ciphertext.fix_base64())?
        .chunks(CHUNK_SIZE * 2)
        .enumerate()
        .rev()
        .skip(1)
    {
        println!("chunk #: {}", n);
        let mut intermediate = [0u8; 16];

        for byte in 1..=CHUNK_SIZE {
            println!("byte #: {}", byte);

            for value in 0u8..=255 {
                std::thread::scope(|s| {
                    s.spawn(|| {
                        intermediate.clone_from_slice(&chunks[0..CHUNK_SIZE]);
                        intermediate[16 - byte] = value;
                    });
                });
            }
        }
    }

    Ok(())
}

fn test_byte(client: &Client) {}

impl ModBase64 for &str {
    type Output = String;
    fn fix_base64(self) -> Self::Output {
        self.replace('~', "=").replace('!', "/").replace('-', "+")
    }

    fn fuckup_base64(self) -> Self::Output {
        self.replace('=', "~").replace('/', "!").replace('+', "-")
    }
}
