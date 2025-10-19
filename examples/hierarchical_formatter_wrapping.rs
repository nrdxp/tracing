// The `tracing` crate provides the core API for structured, event-based
// diagnostics.
use tracing::info;
// The `tracing_subscriber` crate provides implementations of the `Subscriber`
// trait, which is used to collect and process `tracing` data.
use tracing_subscriber::{fmt, fmt::format::hierarchical::Hierarchical, prelude::*};

fn main() {
    // println!("=== Fixed Width Wrapping (60 chars) ===");
    demonstrate_fixed_width();

    println!("\n=== Terminal Width Wrapping (auto-detect) ===");
    demonstrate_terminal_width();
}

/// Demonstrates fixed-width text wrapping at 60 characters.
///
/// This approach uses `with_wrap_width(60)` to set a specific width.
/// Use this when you want consistent formatting regardless of terminal size,
/// such as in log files or when you need predictable output width.
fn demonstrate_fixed_width() {
    // Configure hierarchical formatter with fixed text wrapping at 60 characters
    let format = fmt::format()
        .with_ansi(true)
        .with_hierarchical(Hierarchical::default().with_wrap_width(60));

    // Set up subscriber
    let subscriber =
        tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().event_format(format));

    tracing_subscriber::util::SubscriberInitExt::try_init(subscriber).ok();

    // Log events with deeply nested, long strings that demonstrate text wrapping
    info!(
        user.profile.details = "This is a very long string that should wrap nicely at the configured width of 60 characters, demonstrating how the hierarchical formatter handles text wrapping in deeply nested structures.",
        http.request.headers.authorization = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        http.request.headers.user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        http.request.body = "This is a long request body that contains detailed information about the user's request and should definitely wrap when displayed in the hierarchical format.",
        response.metadata.server_info = "Apache/2.4.41 (Ubuntu) OpenSSL/1.1.1f PHP/7.4.3 with additional modules and configurations that make this server information quite lengthy",
        response.metadata.processing_time = "The server took approximately 150 milliseconds to process this request, which includes database queries, business logic execution, and response formatting.",
        user.session.context = "This session context contains information about the user's current state, preferences, and activity history that spans multiple lines when properly formatted.",
        application.config.database.connection_string = "postgresql://username:password@localhost:5432/my_database?sslmode=require&application_name=my_app&connect_timeout=10",
        application.config.cache.redis_url = "redis://username:password@redis-cluster.example.com:6379/0?sentinel_master_name=mymaster&sentinel_nodes=redis-node1:26379,redis-node2:26379",
        monitoring.metrics.request_count = 42,
        monitoring.metrics.error_rate = "0.05%",
        monitoring.metrics.average_response_time = "The average response time across all endpoints is approximately 125 milliseconds, which is within our acceptable performance thresholds."
    );
}

/// Demonstrates automatic terminal width detection for text wrapping.
///
/// This approach uses `with_terminal_width()` to automatically detect the
/// terminal width and wrap text accordingly. Use this when you want the output
/// to adapt to different terminal sizes, providing optimal readability in
/// interactive terminal sessions. Falls back to 80 characters if terminal
/// width cannot be detected.
fn demonstrate_terminal_width() {
    // Configure hierarchical formatter with automatic terminal width detection
    let format = fmt::format()
        .with_ansi(true)
        .with_hierarchical(Hierarchical::default().with_terminal_width());

    // Set up subscriber
    let subscriber =
        tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().event_format(format));

    tracing_subscriber::util::SubscriberInitExt::try_init(subscriber).ok();

    // Log events with deeply nested, long strings that demonstrate text wrapping
    // The wrapping width will be automatically determined based on terminal size
    info!(
        user.profile.details = "This is a very long string that will wrap at the detected terminal width, automatically adapting to different terminal sizes for optimal readability in interactive sessions.",
        http.request.headers.authorization = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        http.request.headers.user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
        http.request.body = "This request body demonstrates terminal-width wrapping, where the text automatically adjusts to fit the current terminal dimensions for better readability.",
        response.metadata.server_info = "Apache/2.4.41 (Ubuntu) OpenSSL/1.1.1f PHP/7.4.3 with additional modules and configurations that make this server information quite lengthy when displayed",
        response.metadata.processing_time = "The server processing time includes database operations, business logic, and response formatting, all measured in milliseconds for performance monitoring.",
        user.session.context = "Session context information includes user preferences, activity history, and current state that adapts to terminal width for optimal display formatting.",
        application.config.database.connection_string = "postgresql://username:password@localhost:5432/my_database?sslmode=require&application_name=my_app&connect_timeout=10",
        application.config.cache.redis_url = "redis://username:password@redis-cluster.example.com:6379/0?sentinel_master_name=mymaster&sentinel_nodes=redis-node1:26379,redis-node2:26379",
        monitoring.metrics.request_count = 42,
        monitoring.metrics.error_rate = "0.05%",
        monitoring.metrics.average_response_time = "Average response time across all endpoints is approximately 125 milliseconds, within acceptable performance thresholds for this application."
    );

    // Additional example with different nested structures
    info!(
        api.gateway.request.id = "req-1234567890-abcdef",
        api.gateway.request.endpoint = "/api/v1/users/profile/update",
        api.gateway.request.method = "PUT",
        api.gateway.request.payload = "This payload contains user profile update information including personal details, preferences, and settings that need to be processed and validated before updating the database records with terminal-width wrapping.",
        api.gateway.response.status_code = 200,
        api.gateway.response.headers.content_type = "application/json",
        api.gateway.response.body = "The response body includes the updated user profile information, confirmation messages, and any additional metadata about the successful update operation that was performed with automatic width detection.",
        database.transaction.id = "txn-abcdef1234567890",
        database.transaction.queries = "Multiple SQL queries were executed including SELECT for user validation, UPDATE for profile changes, and INSERT for audit logging, all wrapped in a single database transaction with terminal-aware formatting.",
        cache.operations.reads = 15,
        cache.operations.writes = 3,
        cache.operations.hits = "Cache hit rate is currently at 85%, which indicates good performance for frequently accessed data that doesn't change often and adapts to terminal width.",
        security.audit.event_type = "profile_update",
        security.audit.user_id = "user-12345",
        security.audit.ip_address = "192.168.1.100",
        security.audit.user_agent = "This is a very long user agent string that contains browser information, operating system details, and various other metadata about the client making the request with terminal-width text wrapping.",
        security.audit.session_id = "session-abcdef1234567890",
        security.audit.timestamp = "2023-10-19T18:37:23.365Z",
        security.audit.changes = "The following changes were made to the user profile: email address updated, phone number added, notification preferences modified, and privacy settings adjusted with automatic terminal detection."
    );
}
