//! Messaging apps' databases for forensics, read without SQLite: who
//! talked to whom, when and what was said, the calls, SMS and files sent.
//!
//! - Skype's `main.db` (classic Skype, up to version 7): the account,
//!   its contacts, chats and messages, calls with their members, SMS and
//!   file transfers ([`read_skype`]).
//!
//! Columns are read by name: one a version lacks reads as `None`, one it
//! added is ignored. The `-wal` file's committed changes are applied.
//! Damage goes to `problems`, never a panic.
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let skype = messengers::read_skype(&std::fs::read("main.db")?, &[])?;
//! for message in &skype.messages {
//!     println!("{:?} {:?}: {:?}", message.time, message.author, message.body);
//! }
//! # Ok(())
//! # }
//! ```

mod skype;
mod table;

use sqlite::Database;

pub use skype::{Account, Call, CallMember, Chat, Contact, Message, Skype, Sms, Transfer};

/// This crate's version, for records of what parsed them.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Which app's database a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Skype's `main.db` (tables `Accounts`, `Messages`, `Calls`, …).
    Skype,
}

impl Kind {
    /// The kind an open database is, from its tables.
    #[must_use]
    pub fn of(db: &Database<'_>) -> Option<Self> {
        let has = |table: &str| db.table(table).is_some();
        (has("Accounts") && has("Messages") && has("Calls") && has("Transfers"))
            .then_some(Self::Skype)
    }
}

/// Which app's database a file is, from its tables; `None` for anything
/// else.
#[must_use]
pub fn detect(data: &[u8]) -> Option<Kind> {
    if !data.starts_with(b"SQLite format 3\0") {
        return None;
    }
    Database::open(data).ok().and_then(|db| Kind::of(&db))
}

/// Why a file can't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Read a Skype `main.db` with its `-wal` file's committed changes (`wal`
/// may be empty).
///
/// # Errors
/// When it isn't a SQLite database with Skype's tables.
pub fn read_skype(database: &[u8], wal: &[u8]) -> Result<Skype, Error> {
    let db = Database::open_with_wal(database, wal).map_err(|e| Error(e.to_string()))?;
    if Kind::of(&db) != Some(Kind::Skype) {
        return Err(Error("not a Skype main.db".to_owned()));
    }
    let mut problems = db.problems.clone();
    let mut skype = skype::read(&db, &mut problems);
    skype.problems = problems;
    Ok(skype)
}
