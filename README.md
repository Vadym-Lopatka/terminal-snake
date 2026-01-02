# 🐍 Terminal Snake

A classic Snake game that runs in your terminal, built with Rust using `ratatui` and `crossterm`.

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- Clean terminal UI with centered game board
- WASD controls
- Progressive difficulty — snake speeds up as it grows
- Configurable grid size and game parameters
- Game over screen with final score
- Works on macOS, Linux, and Windows

## Quick Start

```bash
git clone https://github.com/Vadym-Lopatka/terminal-snake.git
cd snaker
cargo run --release
```

## Controls

| Key | Action |
|-----|--------|
| `W` | Move up |
| `A` | Move left |
| `S` | Move down |
| `D` | Move right |
| `ESC` | Quit game |

## Configuration

Edit the constants at the top of `src/main.rs` to customize the game:

```rust
const GRID_WIDTH: u16 = 20;          // Board width in cells
const GRID_HEIGHT: u16 = 20;         // Board height in cells
const INITIAL_SNAKE_LENGTH: usize = 3;
const BASE_TICK_MS: u64 = 200;       // Starting speed (lower = faster)
const MIN_TICK_MS: u64 = 50;         // Maximum speed cap
const SPEED_INCREASE_PER_FOOD: u64 = 5; // Speed increase per food eaten
```

## Requirements

- Rust 1.70 or later
- A terminal with UTF-8 support

## Dependencies

- [ratatui](https://github.com/ratatui-org/ratatui) — Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — Cross-platform terminal manipulation
- [rand](https://github.com/rust-random/rand) — Random number generation

## Project Structure

```
snake_game/
├── Cargo.toml
└── src/
    └── main.rs    # All game logic in a single file
```

The entire game is contained in one file (~380 lines) for simplicity and readability.

## How It Works

1. **Game Loop**: Uses a tick-based system where the snake moves at regular intervals
2. **Input Handling**: Non-blocking input polling with direction queuing to prevent 180° turns
3. **Collision Detection**: Checks for wall and self-collision each tick
4. **Rendering**: Redraws the entire game state each frame using ratatui's immediate mode rendering

## License

MIT

## Contributing

PRs welcome! Some ideas for improvements:

- [ ] Add arrow key support
- [ ] High score persistence
- [ ] Pause functionality
- [ ] Different game modes (wrap-around walls, obstacles)
- [ ] Color themes
