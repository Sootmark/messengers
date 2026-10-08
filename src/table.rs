//! Rows read by column name, so that a column a version lacks, or one it
//! added, changes nothing: a missing column reads as NULL.

use std::rc::Rc;

use sqlite::{Database, Value};

/// A row with its table's column names.
pub(crate) struct Named {
    columns: Rc<Vec<String>>,
    pub(crate) rowid: i64,
    values: Vec<Value>,
}

impl Named {
    fn value(&self, column: &str) -> Option<&Value> {
        let at = self
            .columns
            .iter()
            .position(|name| name.eq_ignore_ascii_case(column))?;
        self.values.get(at)
    }

    /// The column's integer; `None` when it is NULL, not an integer, or
    /// not in the table.
    pub(crate) fn integer(&self, column: &str) -> Option<i64> {
        self.value(column).and_then(Value::as_integer)
    }

    /// The column's text; `None` when it is NULL, not text, or not in the
    /// table.
    pub(crate) fn text(&self, column: &str) -> Option<String> {
        self.value(column)
            .and_then(Value::as_text)
            .map(str::to_owned)
    }

    /// The column's integer, or its text read as a decimal integer.
    pub(crate) fn number(&self, column: &str) -> Option<i64> {
        match self.value(column)? {
            Value::Text(text) => text.trim().parse().ok(),
            other => other.as_integer(),
        }
    }
}

/// Every row of `table` in rowid order; none when the table is absent.
/// Damage met on the way is added to `problems`.
pub(crate) fn rows(db: &Database<'_>, table: &str, problems: &mut Vec<String>) -> Vec<Named> {
    let Some(columns) = db.table(table).map(|t| {
        t.column_names()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    }) else {
        return Vec::new();
    };
    let columns = Rc::new(columns);
    let mut rows = match db.rows(table) {
        Ok(rows) => rows,
        Err(e) => {
            problems.push(format!("{table}: {e}"));
            return Vec::new();
        }
    };
    let named = rows
        .by_ref()
        .map(|row| Named {
            columns: Rc::clone(&columns),
            rowid: row.rowid,
            values: row.values,
        })
        .collect();
    problems.extend(rows.problems().iter().map(|p| format!("{table}: {p}")));
    named
}
