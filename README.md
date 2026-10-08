# messengers

Messaging apps' databases for forensics, read without SQLite: who talked to whom, when and what was said, the calls, SMS and files sent. Skype's `main.db` (classic Skype, up to version 7) for now. Two dependencies, its siblings `sootmark-common` (times) and `sootmark-sqlite`.

```toml
[dependencies]
sootmark-messengers = "0.1"
```

```rust
let skype = messengers::read_skype(&std::fs::read("main.db")?, &[])?;
for message in &skype.messages {
    println!("{:?} {:?}: {:?}", message.time, message.author, message.body);
}
for transfer in &skype.transfers {
    println!("{:?} {:?} {:?}", transfer.start, transfer.partner_handle, transfer.path);
}
```

## What you get

- `read_skype(database, wal)`: a `Skype` with the `-wal` file's committed changes applied:
  - `accounts`: Skype name, full and display names, e-mail addresses, country, and when the profile and mood last changed, the account was last online and used, and authorization was last requested and sent;
  - `contacts`: Skype name, names, country, city, phones, e-mail addresses, last online, profile change;
  - `chats`: name, title, participants, dialog partner, creation; `messages`: chat, author and display name, time, body (`body_xml` as stored);
  - `calls`: start, direction, host, duration, and their `members` (identity, display name, GUID, when each joined and for how long, video status, status; `end()` from start and duration);
  - `sms`: numbers, time, body, status;
  - `transfers`: type (incoming, outgoing), partner, status, when offered, accepted and finished, the file's path, name and size, and the `parent_id`/`pk_id` that link a transfer offered to several partners to each partner's copy.
- `detect(bytes)` and `Kind::of(&database)`: whether a database is Skype's.
- Columns are read by name: one a version lacks reads as `None`. Damage goes to `problems`, never a panic; call members whose call is gone are reported.

## Not yet

- Skype 8 and later (an Electron app keeping its data in LevelDB and IndexedDB), Teams, and other messengers.
- Skype's `Conversations`, `Participants`, `Videos`, `Voicemails` and `Alerts` tables.

## How it's checked

- plaso's `skype_main.db` (Apache-2.0, `tests/fixtures/plaso/`): every one of the 24 events plaso's `sqlite/skype` plugin makes (an account time, 15 chat messages, a call's three times, an SMS, four transfer times), read the same (`tests/plaso.rs` against `tests/oracle/plaso.tsv`, from plaso 20260720, written by `tests/oracle/plaso_tsv.py`; the commands in `tests/oracle/README`), plaso's senders, recipients and callers built from this crate's rows.
- Property tests: arbitrary bytes and the database damaged and cut anywhere never panic.

## Licence

MIT or Apache-2.0, at your option. The plaso test file is under the Apache licence 2.0 (`tests/fixtures/plaso/LICENSE`, `NOTICE`).
