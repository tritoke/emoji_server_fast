use anyhow::{bail, Result};
use rand::prelude::*;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use triple_accel::levenshtein::levenshtein;

const REMOTE: &str = "157.245.28.90:12000";

fn main() -> Result<()> {
    println!("Loading words...");
    let words = load_words();
    println!("Making Levenshtein distance cache...");
    let cache = make_distance_cache(&words);

    println!("Making good guesses...");
    'outer: loop {
        let mut server = Server::new()?;
        loop {
            let mut player = Player::new(&words, &cache, &mut server);
            if let Some(flag) = player.play_round()? {
                if flag.ends_with('}') {
                    println!("Found flag: {}", flag);
                    break 'outer;
                }
            } else {
                break;
            }
        }
    }

    Ok(())
}

fn load_words() -> Vec<&'static str> {
    include_str!("../words.txt").lines().collect()
}

#[derive(Debug)]
struct Server {
    stream: TcpStream,
    buf: [u8; 1024],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Response {
    Order(Ordering),
    Correct(String),
    OutOfGuesses,
}

impl Server {
    fn new() -> io::Result<Self> {
        Ok(Self {
            stream: TcpStream::connect(REMOTE)?,
            buf: [0; 1024],
        })
    }

    fn guess(&mut self, word: &str) -> Result<Response> {
        writeln!(self.stream, "{}", word)?;

        let bytes_read = self.stream.read(&mut self.buf)?;
        let resp = std::str::from_utf8(&self.buf[..bytes_read])?;

        Ok(match resp.chars().next() {
            Some('🥵') => Response::Order(Ordering::Less),
            Some('🥶') => Response::Order(Ordering::Greater),
            Some('😐') => Response::Order(Ordering::Equal),
            Some('🥳') => {
                print!("{}", resp);
                if let Some((_, flag)) = resp.split_once(' ') {
                    Response::Correct(String::from(flag.trim()))
                } else {
                    bail!("Got 🥳 but not flag? resp = \"{:?}\"", resp)
                }
            }
            None => Response::OutOfGuesses,
            _ => bail!("AHHHHH resp = {:?}", resp),
        })
    }
}

fn make_distance_cache<'a>(words: &[&'a str]) -> HashMap<(&'a str, &'a str), u32> {
    // this combination of parallel iterators is fastest on my machine
    words
        .par_iter()
        .flat_map_iter(|&a| {
            words
                .iter()
                .map(move |&b| ((a, b), levenshtein(a.as_bytes(), b.as_bytes())))
        })
        .collect()
}

#[derive(Debug)]
struct Player<'a, 'b> {
    words: Vec<&'a str>,
    cache: &'b HashMap<(&'a str, &'a str), u32>,
    server: &'b mut Server,
    previous_guess: Option<&'a str>,
}

impl<'a, 'b> Player<'a, 'b> {
    fn new(words: &[&'a str], cache: &'b HashMap<(&'a str, &'a str), u32>, server: &'b mut Server) -> Self {
        Self {
            words: words.to_vec(),
            cache,
            server,
            previous_guess: None
        }
    }

    fn filter_words(&mut self, guess: &'a str, ord: Ordering) {
        if let Some(previous_guess) = self.previous_guess {
            self.words.retain(|&word| {
                let prev = self.cache[&(word, previous_guess)];
                let curr = self.cache[&(word, guess)];

                curr.cmp(&prev) == ord
            });
        }
    }

    fn pick_word(&self) -> &'a str {
        if let Some(previous_guess) = self.previous_guess {
            self.words
                .par_iter()
                .min_by_key(|&&guess| {
                    let mut less = 0;
                    let mut equal = 0;
                    let mut greater = 0;

                    for test_word in &self.words {
                        let curr = self.cache[&(*test_word, guess)];
                        let prev = self.cache[&(*test_word, previous_guess)];

                        use Ordering::*;
                        match prev.cmp(&curr) {
                            Less => less += 1,
                            Equal => equal += 1,
                            Greater => greater += 1,
                        }
                    }

                    less.max(equal).max(greater)
                })
                .unwrap()
        } else {
            let mut rng = rand::thread_rng();
            self.words.choose(&mut rng).unwrap()
        }
    }

    fn play_round(&mut self) -> Result<Option<String>> {
        Ok(loop {
            let guess = self.pick_word();

            match self.server.guess(guess)? {
                Response::Order(ord) => self.filter_words(guess, ord),
                Response::Correct(flag) => break Some(flag),
                Response::OutOfGuesses => break None,
            }

            self.previous_guess = Some(guess);
        })
    }
}
