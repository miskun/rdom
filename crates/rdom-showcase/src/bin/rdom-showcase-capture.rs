//! The showcase as a capture session over stdio (`rdom_showcase::capture`):
//! JSON-RPC requests on stdin, responses on stdout.
//!
//! ```text
//! echo '{"jsonrpc":"2.0","id":1,"method":"describe"}' | cargo run -p rdom-showcase --bin rdom-showcase-capture
//! ```

fn main() -> std::io::Result<()> {
    rdom_capture::serve(rdom_showcase::capture::Showcase)
}
