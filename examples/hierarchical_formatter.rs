// The `tracing` crate provides the core API for structured, event-based
// diagnostics.
use tracing::info;
// The `tracing_subscriber` crate provides implementations of the `Subscriber`
// trait, which is used to collect and process `tracing` data.
use tracing_subscriber::{fmt, prelude::*};

fn main() {
    // The `hierarchical` formatter is used to display `tracing` events in a
    // tree-like structure.
    let format = fmt::format().with_ansi(true).hierarchical();

    // The `tracing_subscriber` is configured to use the `hierarchical`
    // formatter.
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().event_format(format))
        .init();

    // The `info!` macro is used to record an event. The fields of the event
    // are specified using the `field.name = value` syntax. The `.` in the
    // field names is used to create a hierarchy of fields.
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
}
