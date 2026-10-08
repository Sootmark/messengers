//! Skype's `main.db` (classic Skype, up to version 7, one per account in
//! `%APPDATA%\Skype\<account>\` or `~/Library/Application Support/Skype/<account>/`),
//! a SQLite database. Times are Unix seconds; 0 means none.
//!
//! - `Accounts`: the account's Skype name, full and display names, e-mail
//!   addresses, country, and when its profile and mood last changed, it
//!   was last online and used, and authorization was last requested and
//!   sent.
//! - `Contacts`: each contact's Skype name, names, country, city, phones,
//!   e-mail addresses, when last online and when its profile changed.
//! - `Chats`: each chat's name (`#alice/$bob;…`), title (`friendlyname`),
//!   participants (space-separated), dialog partner and creation time;
//!   `Messages`: each message's chat (`chatname`), author and display name,
//!   time and body (`body_xml`, Skype's XML with entities).
//! - `Calls`: each call's start, direction, host and duration; its
//!   `CallMembers` (`call_db_id`): each member's identity, display name,
//!   GUID (`caller-callee-…`), when the member joined and for how long,
//!   video status (3 on) and status.
//! - `SMSes`: each SMS sent through Skype: its numbers, time and body.
//! - `Transfers`: each file transfer: its type (1 incoming, 2 outgoing),
//!   partner, status, when started, accepted and finished, the file's path,
//!   name and size (text), and `parent_id` and `pk_id`, which link a
//!   transfer offered to several partners to each partner's.

use std::collections::HashMap;

use common::time::Ts;
use sqlite::Database;

use crate::table::{rows, Named};

/// What a Skype `main.db` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Skype {
    /// The account (usually one), in row order.
    pub accounts: Vec<Account>,
    /// Its contacts, in row order.
    pub contacts: Vec<Contact>,
    /// Its chats, in row order.
    pub chats: Vec<Chat>,
    /// Its messages, in row order.
    pub messages: Vec<Message>,
    /// Its calls, with their members, in row order.
    pub calls: Vec<Call>,
    /// Its SMS, in row order.
    pub sms: Vec<Sms>,
    /// Its file transfers, in row order.
    pub transfers: Vec<Transfer>,
    /// Damage met, and rows that don't join.
    pub problems: Vec<String>,
}

/// The account's own profile (`Accounts`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// The row id (`id`).
    pub id: i64,
    /// Its Skype name.
    pub skypename: Option<String>,
    /// Its full name.
    pub fullname: Option<String>,
    /// The name it shows (`given_displayname`).
    pub display_name: Option<String>,
    /// Its e-mail addresses, as written.
    pub emails: Option<String>,
    /// Its country (`us`).
    pub country: Option<String>,
    /// When its profile last changed.
    pub profile_changed: Option<Ts>,
    /// When authorization was last requested of it.
    pub auth_requested: Option<Ts>,
    /// When it last sent an authorization request.
    pub auth_request_sent: Option<Ts>,
    /// When it was last online.
    pub last_online: Option<Ts>,
    /// When it was last used.
    pub last_used: Option<Ts>,
    /// When its mood last changed.
    pub mood_changed: Option<Ts>,
}

/// A contact (`Contacts`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contact {
    /// The row id.
    pub id: i64,
    /// Its Skype name.
    pub skypename: Option<String>,
    /// Its full name.
    pub fullname: Option<String>,
    /// The name it shows.
    pub display_name: Option<String>,
    /// Its country.
    pub country: Option<String>,
    /// Its city.
    pub city: Option<String>,
    /// Its phones (home, office, mobile), those set.
    pub phones: Vec<String>,
    /// Its e-mail addresses, as written.
    pub emails: Option<String>,
    /// When it was last online.
    pub last_online: Option<Ts>,
    /// When its profile last changed.
    pub profile_changed: Option<Ts>,
}

/// A chat (`Chats`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chat {
    /// The row id.
    pub id: i64,
    /// Its name (`#alice/$bob;72abdaf9cf5da36`), which messages name.
    pub name: String,
    /// Its title (`friendlyname`).
    pub title: Option<String>,
    /// Its participants' Skype names.
    pub participants: Vec<String>,
    /// The other party of a one-to-one chat.
    pub dialog_partner: Option<String>,
    /// When it was made.
    pub created: Option<Ts>,
}

/// A chat message (`Messages`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    /// The row id.
    pub id: i64,
    /// Its chat's name ([`Chat::name`]).
    pub chatname: Option<String>,
    /// Its author's Skype name.
    pub author: Option<String>,
    /// Its author's display name, then (`from_dispname`).
    pub from_display_name: Option<String>,
    /// When it was sent.
    pub time: Option<Ts>,
    /// Its body, as stored (`body_xml`: Skype's XML, entities and all).
    pub body: Option<String>,
}

/// A call (`Calls`) and its members (`CallMembers`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Call {
    /// The row id.
    pub id: i64,
    /// When it began.
    pub begin: Option<Ts>,
    /// Whether it came in.
    pub incoming: Option<bool>,
    /// The Skype name of who hosted it.
    pub host_identity: Option<String>,
    /// How long it lasted, in seconds.
    pub duration: Option<i64>,
    /// Its members, in row order.
    pub members: Vec<CallMember>,
}

/// A member of a call (`CallMembers`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallMember {
    /// The row id.
    pub id: i64,
    /// Its Skype name or number.
    pub identity: Option<String>,
    /// Its display name.
    pub display_name: Option<String>,
    /// The call's GUID (`caller-callee-…`).
    pub guid: Option<String>,
    /// When it joined.
    pub start: Option<Ts>,
    /// How long it was in, in seconds.
    pub duration: Option<i64>,
    /// Its video status (3: on).
    pub video_status: Option<i64>,
    /// Its status, as Skype numbers it.
    pub status: Option<i64>,
}

impl CallMember {
    /// When it left: its start plus its duration, when both are known.
    #[must_use]
    pub fn end(&self) -> Option<Ts> {
        let start = self.start?.ticks()?;
        let seconds = self.duration.filter(|&d| d != 0)?;
        let ticks = seconds
            .checked_mul(common::time::TICKS_PER_SECOND)
            .and_then(|t| t.checked_add(start))?;
        Some(Ts::from_ticks(ticks, common::time::Precision::Second))
    }
}

/// An SMS sent through Skype (`SMSes`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sms {
    /// The row id.
    pub id: i64,
    /// The numbers it was sent to, as written.
    pub target_numbers: Option<String>,
    /// When.
    pub time: Option<Ts>,
    /// Its text.
    pub body: Option<String>,
    /// Its status, as Skype numbers it.
    pub status: Option<i64>,
}

/// A file transfer (`Transfers`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Transfer {
    /// The row id.
    pub id: i64,
    /// Its type: 1 incoming, 2 outgoing.
    pub kind: Option<i64>,
    /// The other party's Skype name.
    pub partner_handle: Option<String>,
    /// The other party's display name.
    pub partner_display_name: Option<String>,
    /// Its status, as Skype numbers it (8: completed, …).
    pub status: Option<i64>,
    /// When it was offered.
    pub start: Option<Ts>,
    /// When it was accepted.
    pub accepted: Option<Ts>,
    /// When it finished.
    pub finished: Option<Ts>,
    /// The file's path.
    pub path: Option<String>,
    /// The file's name.
    pub filename: Option<String>,
    /// The file's size, in bytes.
    pub size: Option<i64>,
    /// The `pk_id` of the transfer this one is a partner's copy of.
    pub parent_id: Option<i64>,
    /// Its own key, which its partners' copies name as `parent_id`.
    pub pk_id: Option<i64>,
}

/// Every account, contact, chat, message, call, SMS and transfer.
pub(crate) fn read(db: &Database<'_>, problems: &mut Vec<String>) -> Skype {
    let mut members: HashMap<i64, Vec<CallMember>> = HashMap::new();
    for row in rows(db, "CallMembers", problems) {
        let member = call_member(&row);
        match row.integer("call_db_id") {
            Some(call) => members.entry(call).or_default().push(member),
            None => problems.push(format!("CallMembers: row {} names no call", row.rowid)),
        }
    }
    let calls: Vec<Call> = rows(db, "Calls", problems)
        .iter()
        .map(|row| {
            let mut call = call(row);
            call.members = members.remove(&call.id).unwrap_or_default();
            call
        })
        .collect();
    let mut orphans: Vec<i64> = members.into_keys().collect();
    orphans.sort_unstable();
    problems.extend(
        orphans
            .into_iter()
            .map(|call| format!("CallMembers: call {call} is not in Calls")),
    );
    Skype {
        accounts: rows(db, "Accounts", problems).iter().map(account).collect(),
        contacts: rows(db, "Contacts", problems).iter().map(contact).collect(),
        chats: rows(db, "Chats", problems).iter().map(chat).collect(),
        messages: rows(db, "Messages", problems).iter().map(message).collect(),
        calls,
        sms: rows(db, "SMSes", problems).iter().map(sms).collect(),
        transfers: rows(db, "Transfers", problems)
            .iter()
            .map(transfer)
            .collect(),
        problems: Vec::new(),
    }
}

/// Unix seconds; 0 for none.
fn seconds(row: &Named, column: &str) -> Option<Ts> {
    row.integer(column)
        .filter(|&t| t != 0)
        .map(Ts::from_unix_seconds)
}

fn id(row: &Named) -> i64 {
    row.integer("id").unwrap_or(row.rowid)
}

fn account(row: &Named) -> Account {
    Account {
        id: id(row),
        skypename: row.text("skypename"),
        fullname: row.text("fullname"),
        display_name: row.text("given_displayname"),
        emails: row.text("emails"),
        country: row.text("country"),
        profile_changed: seconds(row, "profile_timestamp"),
        auth_requested: seconds(row, "authreq_timestamp"),
        auth_request_sent: seconds(row, "sent_authrequest_time"),
        last_online: seconds(row, "lastonline_timestamp"),
        last_used: seconds(row, "lastused_timestamp"),
        mood_changed: seconds(row, "mood_timestamp"),
    }
}

fn contact(row: &Named) -> Contact {
    Contact {
        id: id(row),
        skypename: row.text("skypename"),
        fullname: row.text("fullname"),
        display_name: row.text("displayname"),
        country: row.text("country"),
        city: row.text("city"),
        phones: ["phone_home", "phone_office", "phone_mobile"]
            .iter()
            .filter_map(|column| row.text(column).filter(|p| !p.is_empty()))
            .collect(),
        emails: row.text("emails"),
        last_online: seconds(row, "lastonline_timestamp"),
        profile_changed: seconds(row, "profile_timestamp"),
    }
}

fn chat(row: &Named) -> Chat {
    Chat {
        id: id(row),
        name: row.text("name").unwrap_or_default(),
        title: row.text("friendlyname"),
        participants: row
            .text("participants")
            .unwrap_or_default()
            .split_ascii_whitespace()
            .map(str::to_owned)
            .collect(),
        dialog_partner: row.text("dialog_partner"),
        created: seconds(row, "timestamp"),
    }
}

fn message(row: &Named) -> Message {
    Message {
        id: id(row),
        chatname: row.text("chatname"),
        author: row.text("author"),
        from_display_name: row.text("from_dispname"),
        time: seconds(row, "timestamp"),
        body: row.text("body_xml"),
    }
}

fn call(row: &Named) -> Call {
    Call {
        id: id(row),
        begin: seconds(row, "begin_timestamp"),
        incoming: row.integer("is_incoming").map(|i| i != 0),
        host_identity: row.text("host_identity"),
        duration: row.integer("duration"),
        members: Vec::new(),
    }
}

fn call_member(row: &Named) -> CallMember {
    CallMember {
        id: id(row),
        identity: row.text("identity"),
        display_name: row.text("dispname"),
        guid: row.text("guid"),
        start: seconds(row, "start_timestamp"),
        duration: row.integer("call_duration"),
        video_status: row.integer("videostatus"),
        status: row.integer("status"),
    }
}

fn sms(row: &Named) -> Sms {
    Sms {
        id: id(row),
        target_numbers: row.text("target_numbers"),
        time: seconds(row, "timestamp"),
        body: row.text("body"),
        status: row.integer("status"),
    }
}

fn transfer(row: &Named) -> Transfer {
    Transfer {
        id: id(row),
        kind: row.integer("type"),
        partner_handle: row.text("partner_handle"),
        partner_display_name: row.text("partner_dispname"),
        status: row.integer("status"),
        start: seconds(row, "starttime"),
        accepted: seconds(row, "accepttime"),
        finished: seconds(row, "finishtime"),
        path: row.text("filepath"),
        filename: row.text("filename"),
        size: row.number("filesize"),
        parent_id: row.integer("parent_id"),
        pk_id: row.integer("pk_id"),
    }
}
