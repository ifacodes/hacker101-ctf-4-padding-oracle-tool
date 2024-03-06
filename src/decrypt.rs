use std::{process::Output, thread::JoinHandle};

use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};

static CHUNK_SIZE: usize = 16;

trait ModBase64 {
    type Output;
    fn fix_base64(self) -> Self::Output;
    fn fuckup_base64(self) -> Self::Output;
}

fn decrypt() -> Result<()> {
    let ciphertext = "";
    let url = "";

    for chunk in BASE64_STANDARD
        .decode(ciphertext.fix_base64())?
        .chunks(CHUNK_SIZE)
        .skip(1)
    {
        let intermediate = [0u8; 16];
        (1..=CHUNK_SIZE as u8).for_each(|chunk| {
            let handles: Vec<JoinHandle<Option<u8>>> = vec![];
            (0u8..=255).for_each(|value| {})
        })
    }

    Ok(())
}

fn test_byte() {}

impl ModBase64 for &str {
    type Output = String;
    fn fix_base64(self) -> Self::Output {
        self.replace('~', "=").replace('!', "/").replace('-', "+")
    }

    fn fuckup_base64(self) -> Self::Output {
        self.replace('=', "~").replace('/', "!").replace('+', "-")
    }
}
