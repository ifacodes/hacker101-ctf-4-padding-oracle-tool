use std::{process::Output, thread::JoinHandle};

use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use rayon::prelude::*;
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
    let ciphertext = BASE64_STANDARD.decode(ciphertext.fix_base64())?;

    rayon::ThreadPoolBuilder::new()
        .num_threads(10)
        .build()
        .unwrap()
        .install(|| {
            for (n, chunks) in ciphertext.chunks(CHUNK_SIZE * 2).enumerate().rev().take(1) {
                println!("chunk #: {}", n);

                for byte in 1..=CHUNK_SIZE {
                    println!("byte #: {}", byte);

                    (0u8..=255)
                        .into_par_iter()
                        .map(|value| {
                            let mut intermediate: [u8; 16] =
                                chunks[0..CHUNK_SIZE].try_into().unwrap();
                            intermediate[16 - byte] = value;
                            intermediate
                        })
                        .find_any(|intermediate| intermediate[0] == 0);
                }
            }
        });

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
