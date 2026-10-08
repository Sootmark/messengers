//! plaso's Skype `main.db` (Apache-2.0, `tests/fixtures/plaso/`,
//! gzip-compressed): every one of the 24 events plaso's `sqlite/skype`
//! plugin makes, read the same (`tests/oracle/plaso.tsv`, written from
//! plaso's output by `tests/oracle/plaso_tsv.py`; see
//! `tests/oracle/README`). The test builds plaso's view from this crate's
//! rows: a chat message's sender as `display name <author>` and its
//! recipients as the chat's other participants, a call's caller and callee
//! from its member's GUID, a transfer's sender and receiver from the copies
//! `parent_id` and `pk_id` link.
//!
//! Where this crate differs from plaso: plaso reads only the messages of
//! chats (all 15 here), the calls with members, and the times that are
//! set; it says a call wasn't started by the account though its
//! `is_incoming` is 0, which isn't compared. This crate reads contacts and
//! chats too, beyond plaso.

use std::io::Read;

use common::time::Ts;
use messengers::{Kind, Skype, Transfer};

fn skype() -> Skype {
    let mut data = Vec::new();
    common::gzip::Decoder::new(&include_bytes!("fixtures/plaso/skype_main.db.gz")[..])
        .read_to_end(&mut data)
        .unwrap();
    assert_eq!(messengers::detect(&data), Some(Kind::Skype));
    let skype = messengers::read_skype(&data, &[]).unwrap();
    assert_eq!(skype.problems, Vec::<String>::new());
    skype
}

fn micros(time: Ts) -> String {
    time.ticks().unwrap().div_euclid(10).to_string()
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn text(value: Option<impl ToString>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

/// The lines of one plaso event data: one per time set.
fn push(lines: &mut Vec<String>, data_type: &str, times: &[(&str, Option<Ts>)], fields: &[String]) {
    for (desc, time) in times {
        if let Some(time) = time {
            let mut values = vec![
                "skype_main.db".to_owned(),
                format!("skype:event:{data_type}"),
                (*desc).to_owned(),
                micros(*time),
            ];
            values.extend(fields.iter().map(|f| escape(f)));
            lines.push(values.join("\t"));
        }
    }
}

/// A transfer's sender and receiver, each a handle and display name, as
/// plaso pairs a copy with the transfer it is a partner's copy of.
fn parties(skype: &Skype, transfer: &Transfer) -> [String; 4] {
    let partner = |t: &Transfer| {
        [
            text(t.partner_handle.as_ref()),
            text(t.partner_display_name.as_ref()),
        ]
    };
    let none = || [String::new(), String::new()];
    let (sender, receiver) = if let Some(parent) = transfer.parent_id.filter(|&p| p != 0) {
        let source = skype.transfers.iter().find(|t| t.pk_id == Some(parent));
        (source.map_or_else(none, partner), partner(transfer))
    } else {
        let copy = transfer
            .pk_id
            .and_then(|pk| skype.transfers.iter().find(|t| t.parent_id == Some(pk)));
        (partner(transfer), copy.map_or_else(none, partner))
    };
    let [a, b] = sender;
    let [c, d] = receiver;
    [a, b, c, d]
}

/// The account's times.
fn account_lines(skype: &Skype, lines: &mut Vec<String>) {
    for account in &skype.accounts {
        push(
            lines,
            "account",
            &[
                ("Profile Change Time", account.profile_changed),
                ("Authentication Request Time", account.auth_requested),
                (
                    "Authentication Request Sent Time",
                    account.auth_request_sent,
                ),
                ("Last Online Time", account.last_online),
                ("Last Used Time", account.last_used),
                ("Mood Change Time", account.mood_changed),
            ],
            &[
                account.id.to_string(),
                text(account.fullname.as_ref()),
                text(account.display_name.as_ref().filter(|n| !n.is_empty())),
                text(account.emails.as_ref()),
                text(account.country.as_ref()),
            ],
        );
    }
}

/// The messages of chats.
fn chat_lines(skype: &Skype, lines: &mut Vec<String>) {
    for message in &skype.messages {
        let Some(chat) = skype
            .chats
            .iter()
            .find(|c| Some(&c.name) == message.chatname.as_ref())
        else {
            continue;
        };
        let author = text(message.author.as_ref());
        let others: Vec<&str> = chat
            .participants
            .iter()
            .map(String::as_str)
            .filter(|p| *p != author)
            .collect();
        let to = if others.is_empty() {
            text(chat.dialog_partner.as_ref())
        } else {
            others.join(", ")
        };
        push(
            lines,
            "chat",
            &[("Recorded Time", message.time)],
            &[
                format!("{} <{author}>", text(message.from_display_name.as_ref())),
                to,
                text(chat.title.as_ref()),
                text(message.body.as_ref()),
            ],
        );
    }
}

/// Each call member's attempt, start and end.
fn call_lines(skype: &Skype, lines: &mut Vec<String>) {
    for call in &skype.calls {
        for member in &call.members {
            let guid = text(member.guid.as_ref());
            let mut ends = guid.split('-');
            let (source, destination) = (ends.next().unwrap(), ends.next().unwrap());
            let accepted = member.end().is_some();
            push(
                lines,
                "call",
                &[
                    ("Attempt Time", call.begin),
                    ("Start Time", member.start),
                    ("End Time", member.end()),
                ],
                &[
                    call.id.to_string(),
                    source.to_owned(),
                    destination.to_owned(),
                    text(member.duration),
                    if accepted { "ACCEPTED" } else { "WAITING" }.to_owned(),
                    if member.video_status == Some(3) {
                        "True"
                    } else {
                        "False"
                    }
                    .to_owned(),
                ],
            );
        }
    }
}

/// The SMS.
fn sms_lines(skype: &Skype, lines: &mut Vec<String>) {
    for sms in &skype.sms {
        push(
            lines,
            "sms",
            &[("Recorded Time", sms.time)],
            &[
                text(sms.target_numbers.as_ref()).replace(' ', ""),
                text(sms.body.as_ref()),
            ],
        );
    }
}

/// The transfers' times.
fn transfer_lines(skype: &Skype, lines: &mut Vec<String>) {
    for transfer in &skype.transfers {
        let [sender, sender_name, receiver, receiver_name] = parties(skype, transfer);
        push(
            lines,
            "transferfile",
            &[
                ("Start Time", transfer.start),
                ("Accept Time", transfer.accepted),
                ("End Time", transfer.finished),
            ],
            &[
                transfer.id.to_string(),
                sender,
                sender_name,
                receiver,
                receiver_name,
                text(transfer.status),
                text(transfer.filename.as_ref()),
                text(transfer.path.as_ref()),
                text(transfer.size),
            ],
        );
    }
}

#[test]
fn every_event_as_plaso_reads_it() {
    let skype = skype();
    let mut lines = Vec::new();
    account_lines(&skype, &mut lines);
    chat_lines(&skype, &mut lines);
    call_lines(&skype, &mut lines);
    sms_lines(&skype, &mut lines);
    transfer_lines(&skype, &mut lines);
    lines.sort();
    let expected: Vec<&str> = include_str!("oracle/plaso.tsv").lines().collect();
    assert_eq!(lines, expected);
}

#[test]
fn beyond_plaso() {
    let skype = skype();
    assert_eq!(skype.accounts[0].skypename.as_deref(), Some("gen.beringer"));
    assert_eq!(skype.contacts.len(), 2);
    assert_eq!(skype.chats.len(), 5);
    assert_eq!(skype.chats[4].participants.len(), 3);
    let call = &skype.calls[0];
    assert_eq!(call.incoming, Some(false));
    assert_eq!(
        call.members[0].identity.as_deref(),
        Some("european.bbq.competitor")
    );
    assert_eq!(skype.transfers[0].kind, Some(2));
}
