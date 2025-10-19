
//! A hierarchical event formatter.
//!
//! This formatter is designed to format `tracing` events and spans in a hierarchical,
//! human-readable way. It is suitable for use in command-line applications and other
//! text-based outputs.
//!
//! # Key Features
//!
//! - **Hierarchical Span Indentation**: Spans are indented to show their nesting,
//!   making it easy to understand the flow of execution.
//! - **Dotted Field Notation**: Fields with names containing dots (e.g., `http.method`)
//!   are automatically converted into a nested structure.
//! - **Clear Value Display**: Primitive types are printed directly, while complex types
//!   are formatted using their `Debug` implementation.
//! - **Customizable Indentation**: The amount of indentation can be configured to suit
//!   your preferences.
//!
//! # Usage
//!
//! To use the hierarchical formatter, you can add it to your `fmt` subscriber like this:
//!
//! ```rust
//! use tracing_subscriber::fmt;
//!
//! let subscriber = fmt::Subscriber::builder()
//!     .event_format(fmt::format().hierarchical())
//!     .finish();
//!
//! tracing::subscriber::set_global_default(subscriber)
//!     .expect("setting default subscriber failed");
//! ```
//!
//! # Example Output
//!
//! ```text
//! INFO my_span
//!   version: "1.0"
//!   "user logged in"
//!   user:
//!     id: 123
//! ```
//!
//! In this example, `my_span` is a span, and the event "user logged in" occurs
//! within it. The `version` field belongs to the span, while the `user.id` field
//! belongs to the event.
//!
//! # Field Formatting Details
//!
//! The formatter handles fields in the following ways:
//!
//! - **`message`**: The `message` field is treated specially. Its value is printed
//!   as the main message of the event, without the "message:" prefix.
//! - **Dotted Keys**: Keys containing dots are turned into a nested hierarchy. For
//!   example, `http.request.method` becomes:
//!   ```text
//!   http:
//!     request:
//!       method: "POST"
//!   ```
//! - **Value Overwrites**: If a key is assigned both a value and a nested structure
//!   (e.g., `http = "foo"` and `http.method = "POST"`), the formatter will create a
//!   special `<value>` key to hold the direct value:
//!   ```text
//!   http:
//!     <value>: "foo"
//!     method: "POST"
//!   ```
//!
//! This ensures that no information is lost, even if the field naming is ambiguous.
use crate::{
    fmt::{
        format::{self, FmtLevel, Format, Writer},
        time::FormatTime,
        FmtContext, FormattedFields,
    },
    registry::LookupSpan,
};
use std::{
    collections::BTreeMap,
    fmt,
    string::{String, ToString},
    vec::Vec,
};
use crate::field::RecordFields;
use tracing_core::{
    field::{Field, Visit},
    span::Record,
    Event, Subscriber,
};

/// A hierarchical event formatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hierarchical {
    indent_amount: usize,
}

impl Default for Hierarchical {
    fn default() -> Self {
        Self { indent_amount: 2 }
    }
}

impl Hierarchical {
    /// Sets the indent amount for nested fields.
    pub fn with_indent_amount(self, indent_amount: usize) -> Self {
        Self {
            indent_amount,
            ..self
        }
    }
}

// === Data Model ===

/// A value in the hierarchical formatter's data model.
#[derive(Debug, Clone)]
pub enum Value {
    /// A leaf value.
    Leaf(String),
    /// A nested node.
    Node(BTreeMap<String, Value>),
}

// === Event Formatting ===

impl<S, N, T> format::FormatEvent<S, N> for Format<Hierarchical, T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> format::FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if let Some(ansi) = self.ansi {
            writer = writer.with_ansi(ansi);
        }

        self.format_timestamp(&mut writer)?;
        let meta = event.metadata();

        if self.display_level {
            let fmt_level = FmtLevel::new(meta.level(), writer.has_ansi_escapes());
            write!(writer, "{} ", fmt_level)?;
        }

        let indent_amount = self.format.indent_amount;
        let mut in_span = false;

        if let Some(span) = ctx.lookup_current() {
            let mut stack = Vec::new();
            let mut current = Some(span);
            while let Some(s) = current {
                let parent = s.parent();
                stack.push(s);
                current = parent;
            }

            for span_ref in stack.iter().rev() {
                in_span = true;
                write!(writer, "{}", span_ref.name())?;
                if let Some(map) = span_ref.extensions().get::<HierarchicalMap>() {
                    if !map.0.is_empty() {
                        format_fields(&mut writer, map.0.clone(), 0, indent_amount)?;
                    } else {
                        writeln!(writer)?;
                    }
                } else {
                    writeln!(writer)?;
                }
            }
        }

        let event_indent = if in_span { indent_amount } else { 0 };
        write!(writer, "{}", " ".repeat(event_indent))?;

        if self.display_target {
            write!(writer, "{}:", meta.target())?;
        }

        let mut visitor = HierarchicalVisitor::new();
        event.record(&mut visitor);
        let (message, fields) = visitor.finish();

        if let Some(message) = message {
            write!(writer, "{}", message)?;
        }

        if !fields.is_empty() {
            // The `event_indent` is the indentation for the message line, but
            // `format_fields` will add its own indentation. We need to subtract
            // one level of indentation.
            let field_indent = event_indent.saturating_sub(indent_amount);
            format_fields(&mut writer, fields, field_indent, indent_amount)?;
        } else {
            writeln!(writer)?;
        }

        Ok(())
    }
}

// === Field Visitor ===

#[derive(Debug)]
pub(crate) struct HierarchicalVisitor {
    message: Option<String>,
    fields: BTreeMap<String, Value>,
}

impl HierarchicalVisitor {
    pub(crate) fn new() -> Self {
        Self {
            message: None,
            fields: BTreeMap::new(),
        }
    }

    pub(crate) fn finish(self) -> (Option<String>, BTreeMap<String, Value>) {
        (self.message, self.fields)
    }

    fn add_leaf(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
            return;
        }

        let mut parts = field.name().split('.').peekable();
        let mut current_node = &mut self.fields;

        while let Some(part) = parts.next() {
            let is_last_part = parts.peek().is_none();

            if is_last_part {
                use std::collections::btree_map::Entry;
                match current_node.entry(part.to_string()) {
                    Entry::Vacant(entry) => {
                        entry.insert(Value::Leaf(value));
                    }
                    Entry::Occupied(mut entry) => {
                        // A key with this name already exists.
                        match entry.get_mut() {
                            Value::Node(node) => {
                                // It's a node, so we're adding a value for the node itself.
                                node.insert("<value>".to_string(), Value::Leaf(value));
                            }
                            Value::Leaf(_) => {
                                // It's a leaf. The last one wins.
                                entry.insert(Value::Leaf(value));
                            }
                        }
                    }
                }
                return;
            } else {
                // This is an intermediate part of the key.
                let entry = current_node
                    .entry(part.to_string())
                    .or_insert_with(|| Value::Node(BTreeMap::new()));

                // "Promote" a leaf to a node if necessary.
                if let Value::Leaf(leaf_val) = entry {
                    let mut new_node = BTreeMap::new();
                    new_node.insert("<value>".to_string(), Value::Leaf(leaf_val.clone()));
                    *entry = Value::Node(new_node);
                }

                if let Value::Node(node) = entry {
                    current_node = node;
                } else {
                    unreachable!("we just ensured this is a node");
                }
            }
        }
    }
}

impl Visit for HierarchicalVisitor {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.add_leaf(field, value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.add_leaf(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.add_leaf(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.add_leaf(field, value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.add_leaf(field, std::format!("{:?}", value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.add_leaf(field, std::format!("{:?}", value));
    }
}

// === Span Field Formatting ===

/// A newtype wrapper for a `BTreeMap` of fields.
#[derive(Debug)]
pub struct HierarchicalMap(pub(crate) BTreeMap<String, Value>);

/// A [`FormatFields`] implementation that formats fields for the hierarchical
/// formatter.
#[derive(Debug)]
pub struct HierarchicalFields {
    _private: (),
}

impl HierarchicalFields {
    /// Returns a new `HierarchicalFields` implementation.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for HierarchicalFields {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> format::FormatFields<'a> for HierarchicalFields {
    fn format_fields<R: RecordFields>(
        &self,
        _writer: Writer<'_>,
        _fields: R,
    ) -> fmt::Result {
        Ok(())
    }

    fn add_fields(&self, _current: &mut FormattedFields<Self>, _fields: &Record<'_>) -> fmt::Result {
        Ok(())
    }
}

// === Formatting Utilities ===

fn format_fields(
    writer: &mut Writer<'_>,
    fields: BTreeMap<String, Value>,
    indent: usize,
    indent_amount: usize,
) -> fmt::Result {
    if fields.is_empty() {
        return writeln!(writer);
    }
    writeln!(writer)?;
    let mut fields_iter = fields.into_iter().peekable();
    while let Some((key, value)) = fields_iter.next() {
        write!(writer, "{}{}:", " ".repeat(indent + indent_amount), key)?;
        match value {
            Value::Leaf(leaf) => writeln!(writer, " {}", leaf)?,
            Value::Node(node) => {
                format_fields(writer, node, indent + indent_amount, indent_amount)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn merge_fields(target: &mut BTreeMap<String, Value>, source: BTreeMap<String, Value>) {
    for (key, value) in source {
        use std::collections::btree_map::Entry;
        match target.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value);
            }
            Entry::Occupied(mut entry) => match (entry.get_mut(), value) {
                (Value::Node(target_node), Value::Node(source_node)) => {
                    merge_fields(target_node, source_node);
                }
                (target_val, source_val) => {
                    *target_val = source_val;
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tracing::{info, subscriber::with_default};

    #[derive(Clone)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }

    impl<'a> fmt::writer::MakeWriter<'a> for BufWriter {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    #[test]
    fn hierarchical_output() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(buf.clone());
        let format = fmt::format()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .hierarchical();
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .event_format(format)
            .finish();

        with_default(subscriber, || {
            info!(
                message = "user logged in",
                http.request.method = "POST",
                http.request.path = "/login",
                http.response.status = 200,
                user.id = 123,
                user.name = "alice",
                "additional_info" = "some value",
                "more.nested.fields" = "another value"
            );
        });

        let output = String::from_utf8(buf.lock().unwrap().to_vec()).unwrap();
        let expected = r#"INFO "user logged in"
  additional_info: "some value"
  http:
    request:
      method: "POST"
      path: "/login"
    response:
      status: 200
  more:
    nested:
      fields: "another value"
  user:
    id: 123
    name: "alice"
"#;
        assert_eq!(
            output.trim(),
            expected.trim(),
            "output:\n{}\nexpected:\n{}",
            output,
            expected
        );
    }

    #[test]
    fn hierarchical_value_overwrite_is_an_error() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(buf.clone());
        let format = fmt::format()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .hierarchical();
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .event_format(format)
            .finish();

        with_default(subscriber, || {
            info!(
                message = "user logged in",
                http.method = "POST",
                http = "this should NOT be overwritten",
            );
        });

        let output = String::from_utf8(buf.lock().unwrap().to_vec()).unwrap();
        let expected = r#"INFO "user logged in"
  http:
    <value>: "this should NOT be overwritten"
    method: "POST"
"#;
        assert_eq!(
            output.trim(),
            expected.trim(),
            "output:\n{}\nexpected:\n{}",
            output,
            expected
        );
    }

    #[test]
    fn hierarchical_overwrite_leaf_with_node() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(buf.clone());
        let format = fmt::format()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .hierarchical();
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .event_format(format)
            .finish();

        with_default(subscriber, || {
            info!(
                message = "user logged in",
                http = "this should NOT be overwritten",
                http.method = "POST",
            );
        });

        let output = String::from_utf8(buf.lock().unwrap().to_vec()).unwrap();
        let expected = r#"INFO "user logged in"
  http:
    <value>: "this should NOT be overwritten"
    method: "POST"
"#;
        assert_eq!(
            output.trim(),
            expected.trim(),
            "output:\n{}\nexpected:\n{}",
            output,
            expected
        );
    }

    #[test]
    fn hierarchical_with_spans() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(buf.clone());
        let format = fmt::format()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .hierarchical();
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .event_format(format)
            .fmt_fields(HierarchicalFields::new())
            .finish();

        with_default(subscriber, || {
            let span = tracing::info_span!("my_span", version = "1.0");
            let _enter = span.enter();
            info!(message = "user logged in", user.id = 123);
        });

        let output = String::from_utf8(buf.lock().unwrap().to_vec()).unwrap();
        let expected = r#"INFO my_span
  version: "1.0"
  "user logged in"
  user:
    id: 123
"#;
        assert_eq!(
            output.trim(),
            expected.trim(),
            "output:\n{}\nexpected:\n{}",
            output,
            expected
        );
    }

    #[test]
    fn hierarchical_with_nested_spans() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = BufWriter(buf.clone());
        let format = fmt::format()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_file(false)
            .with_line_number(false)
            .hierarchical();
        let subscriber = fmt::Subscriber::builder()
            .with_writer(writer)
            .event_format(format)
            .fmt_fields(HierarchicalFields::new())
            .finish();

        with_default(subscriber, || {
            let outer_span = tracing::info_span!("outer", outer_field = "outer_value");
            let _outer_enter = outer_span.enter();
            let inner_span = tracing::info_span!("inner", inner_field = "inner_value");
            let _inner_enter = inner_span.enter();
            info!(message = "user logged in", user.id = 123);
        });

        let output = String::from_utf8(buf.lock().unwrap().to_vec()).unwrap();
        let expected = r#"INFO outer
  outer_field: "outer_value"
inner
  inner_field: "inner_value"
  "user logged in"
  user:
    id: 123
"#;
        assert_eq!(
            output.trim(),
            expected.trim(),
            "output:\n{}\nexpected:\n{}",
            output,
            expected
        );
    }
}
