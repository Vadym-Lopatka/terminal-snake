# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo run --release    # Build and run the game
cargo build --release  # Build only
cargo check            # Quick syntax/type check
cargo clippy           # Lint
cargo fmt              # Format code
```

## Architecture

This is a single-file Rust terminal snake game (~380 lines in `src/main.rs`).

**Core Components:**
- `Game` struct: Contains all game state (snake body as `VecDeque<Position>`, direction, food position, score)
- `GameState` enum: `Playing` or `GameOver`
- `Direction` enum: Movement directions with `opposite()` method to prevent 180° turns

**Game Loop Pattern:**
1. Tick-based movement using `Instant` for timing
2. Non-blocking input via `crossterm::event::poll()` with timeout
3. Direction queuing (`next_direction`) applied on tick to prevent frame-skip reversals
4. Immediate-mode rendering with `ratatui` - full redraw each frame

**Configuration Constants (top of main.rs):**
- `GRID_WIDTH`/`GRID_HEIGHT`: Board dimensions
- `BASE_TICK_MS`/`MIN_TICK_MS`: Speed range
- `SPEED_INCREASE_PER_FOOD`: Difficulty progression

**Dependencies:** ratatui (TUI framework), crossterm (terminal handling), rand (food spawning)


## Development workflow

Do not add Claude as co-author to commit messages
