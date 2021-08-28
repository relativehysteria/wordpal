//! This module handles the database;
//! loading, parsing, writing, etc. etc.

use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::time::{UNIX_EPOCH, SystemTime, Duration};
use std::path::PathBuf;
use crate::rng::Rng;

/// 24 hours in seconds
const DAY: u64 = 86400;

/// The word timeout values (in days)
const TIMEOUT_DELAYS: [u64; 5] = [0, 1, 7, 14, 30];

/// Column delimiter in the database
const DELIMITER: &str = ";; ";


/// This struct keeps track of the open database file and of its internal
/// in-memory representation.
pub struct Database {
    /// The handle to the database file
    pub file: File,

    /// The vector of usable (not timed-out) database entries
    pub usable: Vec<Entry>,

    /// The vector of unusable (timed-out) database entries
    pub unusable: Vec<Entry>,

    /// The RNG used to get random entries from the database
    pub rng: Rng,
}

impl Database {
    /// Opens the database, parses it and returns it
    pub fn open(filename: PathBuf) -> std::io::Result<Self> {
        // Read the contents of the file
        let mut file     = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // And create vectors of entries from the lines of the file
        let numlines     = contents.lines().count();
        let mut usable   = Vec::with_capacity(numlines);
        let mut unusable = Vec::with_capacity(numlines);

        for line in contents.lines() {
            if let Some(entry) = Entry::parse_from_line(line) {
                if entry.timed_out {
                    unusable.push(entry);
                } else {
                    usable.push(entry);
                }
            }
        }

        Ok(Self {
            file,
            usable,
            unusable,
            rng: Rng::new(),
        })
    }

    /// Writes the internal database representation to the file
    pub fn write_db(&mut self) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;

        let entries = self.usable.iter().chain(self.unusable.iter());
        for entry in entries {
            self.file.write_all(format!("{}\n", entry.db_repr()).as_bytes())?;
        }
        Ok(())
    }

    /// Returns a random usable entry and its index in the database.
    /// If all entries are timed out (that is, unusable), `None` is returned.
    pub fn random_entry(&mut self) -> Option<(Entry, usize)> {
        if self.usable.len() == 0 {
            return None;
        }

        let num = self.rng.range(0, (self.usable.len()-1) as u64) as usize;

        if let Some(entry) = self.usable.get(num) {
            Some((entry.clone(), num))
        } else {
            None
        }
    }

    /// Updates the timeout value of the `index`th entry and moves it from the
    /// inner `usable` vec into the `unusable` one.
    /// If `next` is true, `cur_iter` in the entry is incremented.
    /// If it's false, it is decremented.
    pub fn update_timeout(&mut self, index: usize, next: bool) {
        if index >= self.usable.len() {
            return;
        } else {
            self.usable[index].update_timeout(next);
            self.unusable.push(self.usable[index].clone());
            self.usable.swap_remove(index);
        }
    }
}


/// An entry in the database struct
#[derive(Clone, Debug)]
pub struct Entry {
    /// The original word.
    pub word: String,

    /// The translated word.
    pub tr_word: String,

    /// Current timeout delay iteration
    pub cur_iter: usize,

    /// The word can't be used until this time
    pub timeout: u64,

    /// Whether the word is usable already or still on a timeout.
    /// This value isn't really needed - it can be calculated on the run,
    /// but it makes the code prettier and _very slightly_ faster.
    pub timed_out: bool,
}

impl Entry {
    /// Parses a line taken from a textfile and returns a corresponding Entry.
    /// Returns `None` if the entry is timed out or if an error occurs.
    pub fn parse_from_line(line: &str) -> Option<Self> {
        // Extract the elements from the line
        let split               = line.split(DELIMITER);
        let elements: Vec<&str> = split.into_iter().collect();

        let mut word      = String::new();
        let mut tr_word   = String::new();
        let mut cur_iter  = 0;
        let mut timeout   = Duration::from_secs(0);
        let mut timed_out = false;

        // If there's 4 elements, the entry is valid.
        // If there's 2 elements, the entry is new (no time info) but valid.
        if elements.len() != 2 && elements.len() != 4 {
            return None;
        }

        // All entries
        if elements.len() >= 2 {
            word    = elements.get(0)?.to_string();
            tr_word = elements.get(1)?.to_string();
        }

        // Already initialized entries
        if elements.len() == 4 {
            cur_iter = elements.get(2)?.parse::<usize>().ok()?;

            // Try to parse the timeout into an integer and compare it
            // to current time. If it's greater than current time,
            // the word is on a timeout.
            let timeout_parse = elements.get(3)?;
            let timeout_parse = timeout_parse.parse::<u64>().ok()?;
            timeout = Duration::from_secs(timeout_parse);

            if timeout > SystemTime::now().duration_since(UNIX_EPOCH).ok()? {
                timed_out = true;
            }
        }

        Some(Self {
            word,
            tr_word,
            cur_iter,
            timeout: timeout.as_secs(),
            timed_out,
        })
    }

    /// Updates the timeout of this entry.
    /// If `next` is true, `cur_iter` is incremented.
    /// If it's false, it is decremented.
    pub fn update_timeout(&mut self, next: bool) {
        if self.timed_out {
            return;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        // Update `cur_iter` and also don't overflow
        if next {
            if self.cur_iter != TIMEOUT_DELAYS.len()-1 {
                self.cur_iter += 1;
            }
        } else if self.cur_iter != 0 {
            self.cur_iter -= 1;
        }

        // Update `timeout` based on `cur_iter`.
        // `timed_out` is also set to true.
        let timeout = Duration::from_secs(TIMEOUT_DELAYS[self.cur_iter] * DAY);

        self.timeout   = now.as_secs() + timeout.as_secs();
        self.timed_out = true;
    }

    /// Returns the in-database representation of this entry
    pub fn db_repr(&self) -> String {
        format!("{}{}{}{}{}{}{}",
                self.word,     DELIMITER,
                self.tr_word,  DELIMITER,
                self.cur_iter, DELIMITER,
                self.timeout)
    }
}
