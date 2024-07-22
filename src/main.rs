#![feature(let_chains)]
use std::{
    io::{self, prelude::*, BufReader, Stdout},
    process::ExitCode,
};

use lazy_static::lazy_static;
use onig::*;
use std::str;

fn main() -> ExitCode {
    let stdout = io::stdout();
    let mut stdout = io::BufWriter::new(stdout);

    let reader = BufReader::new(io::stdin());

    for line in reader.lines() {
        let line = line.unwrap();

        if line.starts_with("INSERT INTO ") {
            split_insert(&mut stdout, &line).unwrap();
        } else {
            writeln!(stdout, "{}", line).unwrap();
        }
    }

    ExitCode::from(0)
}

lazy_static! {
    static ref RE: Regex = Regex::new(r"^INSERT INTO `[a-z0-9_]+` VALUES(?= \()").unwrap();
}

fn split_insert(stdout: &mut io::BufWriter<Stdout>, str: &str) -> io::Result<()> {
    let values_loc = RE.find(str);
    if values_loc.is_none() {
        writeln!(stdout, "{}", str)?;
        eprintln!(
            "Encountered a line starting with INSERT INTO that didn't match? {}",
            str
        );
        return Ok(());
    }

    let values_loc = values_loc.unwrap();

    let insert_command = &str[0..values_loc.1].as_bytes();
    stdout.write(insert_command)?;
    stdout.write(b"\n")?;

    let str = &str[(values_loc.1 + 1)..];

    let mut values: Vec<(usize, usize)> = Vec::new();

    let mut state = MatchState::WantStart;

    let mut start_idx = 0;

    let mut skip = false;

    for (first, next) in DoubleIterate::new(str.char_indices()) {
        if skip {
            skip = false;
            continue;
        }
        let (i, c) = first;
        match state {
            MatchState::WantStart => {
                if c == '(' {
                    start_idx = i;
                    state = MatchState::WantEnd;
                } else if c == ';' {
                    // finished
                    break;
                }
            }
            MatchState::WantEnd => {
                if c == ')' {
                    state = MatchState::WantCommaSemi;
                } else if c == '\'' {
                    state = MatchState::InQuote(c);
                }
            }
            MatchState::WantCommaSemi => {
                if c == ';' {
                    // finished, dont include this character
                    values.push((start_idx, i));
                    break;
                } else if c == ',' {
                    values.push((start_idx, i + 1));
                    state = MatchState::WantStart;
                }
            }
            MatchState::InQuote(quote) => {
                // if we hit a backslash character or quote,
                // and the next character is a quote,
                // then skip it.
                if (c == '\\' || c == quote)
                    && let Some(next) = next
                    && next.1 == quote
                {
                    skip = true;
                }

                if c == quote {
                    state = MatchState::WantEnd;
                }
            }
        }
    }
    for (start, end) in values {
        writeln!(stdout, "  {}", &str[start..end])?;
    }
    writeln!(stdout, "{}", ";")?;

    Ok(())
}

enum MatchState {
    WantStart,
    WantEnd,
    WantCommaSemi,
    InQuote(char),
}

struct DoubleIterate<I>
where
    I: Iterator,
{
    iter: I,
    next_item: Option<I::Item>,
}

impl<I> DoubleIterate<I>
where
    I: Iterator,
{
    fn new(mut iter: I) -> Self {
        let next_item = iter.next();
        DoubleIterate { iter, next_item }
    }
}

impl<I> Iterator for DoubleIterate<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, Option<I::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_item.is_none() {
            return None;
        }

        let current_item = self.next_item.clone();
        self.next_item = self.iter.next();

        Some((current_item.unwrap(), self.next_item.clone()))
    }
}
