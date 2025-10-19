//! A hierarchical, human-readable event formatter.
//!
//! This formatter is designed to render `tracing` events and spans in a way that
//! emphasizes the hierarchical structure of the data. It is particularly useful
//! for understanding the context of events within nested spans.
//!
//! # Features
//!
//! - **Tree-like Structure:** Fields are displayed in a tree-like structure,
//!   making it easy to see the relationships between them.
//! - **Clear Delimiters:** The formatter uses clear delimiters and indentation to
//!   represent the hierarchy of the data.
//! - **Human-Readable:** The output is designed to be easily readable by humans,
//!   making it suitable for development and debugging.
//!
//! # Example
//!
//! ```rust
//! use tracing::info;
//! use tracing_subscriber::fmt;
//!
//! fn main() {
//!     let format = fmt::format().hierarchical();
//!
//!     tracing_subscriber::fmt()
//!         .event_format(format)
//!         .init();
//!
//!     info!(
//!         http.request.method = "POST",
//!         http.request.path = "/login",
//!         http.response.status = 200,
//!         user.id = 123,
//!         user.name = "alice",
//!         "user logged in"
//!     );
//! }
//! ```//!
//! The output of the above example would be:
//!
//! ```text
//! 2025-10-19T01:43:12Z INFO tracing_subscriber:
//! ├─ http:
//! │  ├─ request:
//! │  │  ├─ method: "POST"
//! │  │  └─ path: "/login"
//! │  └─ response:
//! │     └─ status: 200
//! └─ user:
//!   ├─ id: 123
//!   └─ name: "alice"
//! ```
//!
use crate::{
    field::MakeVisitor,
    fmt::{
        format::{Format, FormatEvent, FormatFields, Writer},
        time::FormatTime,
        FmtContext,
    },
    registry::LookupSpan,
};
use std::collections::BTreeMap;
use std::vec::Vec;
use std::fmt;
use std::string::{String, ToString};
use tracing_core::{field::Visit, Event, Subscriber};

/// A hierarchical, human-readable event formatter.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Hierarchical;

/// A `MakeVisitor` for the `Hierarchical` formatter.
#[derive(Debug, Default)]
pub struct HierarchicalFields {
    // Configuration options like ANSI color support can be added here.
}

/// The visitor produced by `HierarchicalFields`.
#[derive(Debug)]
pub struct HierarchicalVisitor<'a> {
    tree: BTreeMap<String, ValueNode>,
    writer: Writer<'a>,
}

#[derive(Debug)]
enum ValueNode {
    Leaf(String),
    Branch(BTreeMap<String, ValueNode>),
}

impl<'a> HierarchicalVisitor<'a> {
    fn new(writer: Writer<'a>) -> Self {
        Self {
            tree: BTreeMap::new(),
            writer,
        }
    }

    fn add_field(&mut self, key: &str, value: &str) {
        let mut parts = key.split('.').peekable();
        let mut current_branch = &mut self.tree;

        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                // Last part
                current_branch.insert(part.to_string(), ValueNode::Leaf(value.to_string()));
                return;
            }

            current_branch = match current_branch
                .entry(part.to_string())
                .or_insert_with(|| ValueNode::Branch(BTreeMap::new()))
            {
                ValueNode::Branch(branch) => branch,
                ValueNode::Leaf(_) => {
                    // A leaf value was found where a branch was expected.
                    return;
                }
            };
        }
    }
}

impl<'a> Visit for HierarchicalVisitor<'a> {
    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn fmt::Debug) {
        let value_str = std::format!("{:?}", value);
        if field.name() == "message" {
            let msg = value_str.trim_matches('"');
            let mut message_text = String::new();
            // This is a simple parser that handles spaces in quoted values.
            let mut parts = Vec::new();
            let mut current_part = String::new();
            let mut in_quotes = false;
            for c in msg.chars() {
                match c {
                    '"' => in_quotes = !in_quotes,
                    ' ' if !in_quotes => {
                        if !current_part.is_empty() {
                            parts.push(current_part);
                            current_part = String::new();
                        }
                    }
                    _ => current_part.push(c),
                }
            }
            if !current_part.is_empty() {
                parts.push(current_part);
            }

            for part in parts {
                if let Some((key, value)) = part.split_once('=') {
                    self.add_field(key, value);
                } else {
                    if !message_text.is_empty() {
                        message_text.push(' ');
                    }
                    message_text.push_str(&part);
                }
            }

            if !message_text.is_empty() {
                self.add_field("message", &std::format!("\"{}\"", message_text));
            }
        } else {
            self.add_field(field.name(), &value_str);
        }
    }
}

impl<'a> Drop for HierarchicalVisitor<'a> {
    fn drop(&mut self) {
        if !self.tree.is_empty() {
            let _ = format_tree(&mut self.writer, &self.tree, "");
        }
    }
}

fn format_tree(
    writer: &mut Writer<'_>,
    branch: &BTreeMap<String, ValueNode>,
    prefix: &str,
) -> fmt::Result {
    let mut iter = branch.iter().peekable();
    while let Some((key, value)) = iter.next() {
        let is_last = iter.peek().is_none();
        let (connector, child_prefix) = if is_last {
            ("└─", "   ")
        } else {
            ("├─", "│  ")
        };

        match value {
            ValueNode::Leaf(leaf_value) => {
                writeln!(
                    writer,
                    "{}{} {}: {}",
                    prefix, connector, key, leaf_value
                )?;
            }
            ValueNode::Branch(child_branch) => {
                writeln!(writer, "{}{} {}:", prefix, connector, key)?;
                format_tree(
                    writer,
                    child_branch,
                    &std::format!("{}{}", prefix, child_prefix),
                )?;
            }
        }
    }
    Ok(())
}

impl<'a> MakeVisitor<Writer<'a>> for HierarchicalFields {
    type Visitor = HierarchicalVisitor<'a>;

    fn make_visitor(&self, writer: Writer<'a>) -> Self::Visitor {
        HierarchicalVisitor::new(writer)
    }
}

impl<C, N, T> FormatEvent<C, N> for Format<Hierarchical, T>
where
    C: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, C, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();

        let mut timestamp_writer = writer.by_ref();
        self.timer.format_time(&mut timestamp_writer)?;

        writeln!(
            writer,
            " {} {}:",
            meta.level(),
            meta.target()
        )?;

        let mut visitor = HierarchicalFields::default().make_visitor(writer.by_ref());
        event.record(&mut visitor);
        Ok(())
    }
}