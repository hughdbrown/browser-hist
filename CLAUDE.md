# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build and Run
- `cargo build` - Build the project
- `cargo run` - Run the application
- `cargo run -- --help` - Show command line options
- `target/debug/browser-hist [OPTIONS]` - Run the built binary directly

### Testing and Quality
- `cargo test` - Run tests
- `cargo check` - Fast compile check without building
- `cargo clippy` - Run linter

## Architecture

This is a Rust CLI application that searches Chrome browser history on macOS. The application:

1. **Chrome History Access**: Copies Chrome's SQLite history database to a temporary file (required because Chrome locks the original file)
2. **SQLite Query Engine**: Uses rusqlite to query the copied database with flexible filters
3. **Time Conversion**: Handles Chrome's unique timestamp format (microseconds since 1601-01-01)
4. **CLI Interface**: Uses clap for command-line argument parsing

### Key Components

- **Row struct**: Represents a browser history entry with URL, title, visit count, and timestamp
- **SQL Query Builder**: Dynamically constructs SQL queries based on command-line filters
- **Chrome Time Utilities**: Functions to convert between Chrome timestamps and standard date formats
- **Error Handling**: Custom error type wrapping both SQLite and I/O errors

### Database Schema
The application queries Chrome's `urls` table with columns:
- `url` - The visited URL
- `title` - Page title
- `visit_count` - Number of visits
- `last_visit_time` - Chrome timestamp format

### Chrome History Location
`~/Library/Application Support/Google/Chrome/Default/History`