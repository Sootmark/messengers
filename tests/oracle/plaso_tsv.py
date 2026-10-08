"""Writes plaso's events (psort -a -o json_line) as sorted TSV lines.

One line per event: file (relative to the input directory), data type,
timestamp description, timestamp (microseconds; 0 for "Not a time"), then
the attributes FIELDS lists for its data type, in that order. A list is
written joined with " | ". Backslashes, tabs, carriage returns and line
feeds in text are written as \\\\, \\t, \\r and \\n. An attribute plaso
leaves out is empty. Events of a data type FIELDS doesn't list are left out.

    python3 -I plaso_tsv.py <out.jsonl>
"""

import json
import sys

INPUT_DIRECTORY = "/data/in/"

FIELDS = {
    "skype:event:account": (
        "offset",
        "fullname",
        "account_display_name",
        "email",
        "country",
    ),
    "skype:event:call": (
        "offset",
        "src_call",
        "dst_call",
        "duration",
        "call_type",
        "video_conference",
    ),
    "skype:event:chat": ("from_account", "to_account", "title", "text"),
    "skype:event:sms": ("number", "text"),
    "skype:event:transferfile": (
        "offset",
        "sender",
        "sender_display_name",
        "receiver",
        "receiver_display_name",
        "transfer_status",
        "transferred_filename",
        "transferred_file_path",
        "transferred_file_size",
    ),
}


def text(value):
    """A value as text: a list joined, None empty."""
    if value is None:
        return ""
    if isinstance(value, list):
        return " | ".join(text(item) for item in value)
    return str(value)


def escape(value):
    """Text with backslash, tab, CR and LF escaped."""
    return (
        text(value)
        .replace("\\", "\\\\")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
        .replace("\n", "\\n")
    )


def line(event):
    """The TSV line of one event."""
    file_name = event["display_name"].split(INPUT_DIRECTORY, 1)[1]
    values = [
        file_name,
        event["data_type"],
        event["timestamp_desc"],
        event["timestamp"],
    ]
    values.extend(event.get(field) for field in FIELDS[event["data_type"]])
    return "\t".join(escape(value) for value in values)


def main(path):
    with open(path, encoding="utf-8") as json_lines:
        events = [json.loads(text) for text in json_lines if text.strip()]
    lines = sorted(line(event) for event in events if event["data_type"] in FIELDS)
    for text_line in lines:
        print(text_line)


if __name__ == "__main__":
    main(sys.argv[1])
