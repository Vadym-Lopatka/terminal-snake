use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    collections::VecDeque,
    io::{self, stdout},
    time::{Duration, Instant},
};

// ============================================================================
// Configuration
// ============================================================================

const GRID_WIDTH: u16 = 20;
const GRID_HEIGHT: u16 = 20;
const INITIAL_SNAKE_LENGTH: usize = 3;
const BASE_TICK_MS: u64 = 200;
const MIN_TICK_MS: u64 = 50;
const SPEED_INCREASE_PER_FOOD: u64 = 5;

// Symbols
const SNAKE_BODY: &str = "●";
const FOOD_SYMBOL: &str = "●";

// ============================================================================
// Types
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i16,
    y: i16,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

enum GameState {
    Playing,
    GameOver,
}

struct Game {
    snake: VecDeque<Position>,
    direction: Direction,
    next_direction: Direction,
    food: Position,
    score: u32,
    state: GameState,
    grid_width: u16,
    grid_height: u16,
}

// ============================================================================
// Game Logic
// ============================================================================

impl Game {
    fn new(grid_width: u16, grid_height: u16) -> Self {
        let center_x = grid_width as i16 / 2;
        let center_y = grid_height as i16 / 2;

        // Create initial snake (horizontal, facing right)
        let mut snake = VecDeque::new();
        for i in 0..INITIAL_SNAKE_LENGTH {
            snake.push_back(Position {
                x: center_x - i as i16,
                y: center_y,
            });
        }

        let mut game = Game {
            snake,
            direction: Direction::Right,
            next_direction: Direction::Right,
            food: Position { x: 0, y: 0 },
            score: 0,
            state: GameState::Playing,
            grid_width,
            grid_height,
        };

        game.spawn_food();
        game
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        loop {
            let pos = Position {
                x: rng.gen_range(0..self.grid_width as i16),
                y: rng.gen_range(0..self.grid_height as i16),
            };

            // Ensure food doesn't spawn on snake
            if !self.snake.contains(&pos) {
                self.food = pos;
                break;
            }
        }
    }

    fn tick(&mut self) {
        if !matches!(self.state, GameState::Playing) {
            return;
        }

        // Apply the queued direction change
        self.direction = self.next_direction;

        // Calculate new head position
        let head = self.snake.front().unwrap();
        let new_head = match self.direction {
            Direction::Up => Position { x: head.x, y: head.y - 1 },
            Direction::Down => Position { x: head.x, y: head.y + 1 },
            Direction::Left => Position { x: head.x - 1, y: head.y },
            Direction::Right => Position { x: head.x + 1, y: head.y },
        };

        // Check wall collision
        if new_head.x < 0
            || new_head.x >= self.grid_width as i16
            || new_head.y < 0
            || new_head.y >= self.grid_height as i16
        {
            self.state = GameState::GameOver;
            return;
        }

        // Check self collision
        if self.snake.contains(&new_head) {
            self.state = GameState::GameOver;
            return;
        }

        // Move snake
        self.snake.push_front(new_head);

        // Check food collision
        if new_head == self.food {
            self.score += 1;
            self.spawn_food();
            // Don't remove tail - snake grows
        } else {
            self.snake.pop_back();
        }
    }

    fn change_direction(&mut self, new_direction: Direction) {
        // Prevent 180-degree turns
        if new_direction != self.direction.opposite() {
            self.next_direction = new_direction;
        }
    }

    fn tick_duration(&self) -> Duration {
        let speed_reduction = self.score as u64 * SPEED_INCREASE_PER_FOOD;
        let tick_ms = BASE_TICK_MS.saturating_sub(speed_reduction).max(MIN_TICK_MS);
        Duration::from_millis(tick_ms)
    }
}

// ============================================================================
// Rendering
// ============================================================================

fn render(frame: &mut Frame, game: &Game) {
    let area = frame.size();

    match game.state {
        GameState::Playing => render_game(frame, game, area),
        GameState::GameOver => render_game_over(frame, game, area),
    }
}

fn render_game(frame: &mut Frame, game: &Game, area: Rect) {
    // Calculate the size needed for the game grid
    // Each cell is 2 characters wide for better aspect ratio
    let grid_width = game.grid_width * 2 + 2; // +2 for borders
    let grid_height = game.grid_height + 2; // +2 for borders

    // Center the game grid
    let game_area = centered_rect(grid_width, grid_height, area);

    // Create the game board
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Snake - Score: {} ", game.score))
        .title_alignment(Alignment::Center);

    let inner = block.inner(game_area);
    frame.render_widget(block, game_area);

    // Render grid contents
    let mut lines: Vec<Line> = Vec::new();

    for y in 0..game.grid_height {
        let mut spans: Vec<Span> = Vec::new();

        for x in 0..game.grid_width {
            let pos = Position { x: x as i16, y: y as i16 };
            let (symbol, style) = if game.snake.front() == Some(&pos) {
                // Snake head
                (SNAKE_BODY, Style::default().fg(Color::Green))
            } else if game.snake.contains(&pos) {
                // Snake body
                (SNAKE_BODY, Style::default().fg(Color::LightGreen))
            } else if pos == game.food {
                // Food
                (FOOD_SYMBOL, Style::default().fg(Color::Red))
            } else {
                // Empty cell
                ("  ", Style::default())
            };

            // Use 2 chars per cell for better aspect ratio
            let display = if symbol == "  " {
                "  ".to_string()
            } else {
                format!("{} ", symbol)
            };
            spans.push(Span::styled(display, style));
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    // Render controls hint below the game
    let controls_area = Rect {
        x: area.x,
        y: game_area.y + game_area.height,
        width: area.width,
        height: 1,
    };

    if controls_area.y < area.height {
        let controls = Paragraph::new("WASD: Move | ESC: Quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(controls, controls_area);
    }
}

fn render_game_over(frame: &mut Frame, game: &Game, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "GAME OVER",
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(format!("Final Score: {}", game.score)),
        Line::from(""),
        Line::from(Span::styled(
            "Press ESC to quit",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Game Over ")
                .title_alignment(Alignment::Center),
        );

    let popup_area = centered_rect(30, 10, area);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width.min(area.width)),
        Constraint::Fill(1),
    ])
    .split(area);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height.min(area.height)),
        Constraint::Fill(1),
    ])
    .split(horizontal[1]);

    vertical[1]
}

// ============================================================================
// Main Loop
// ============================================================================

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Create game
    let mut game = Game::new(GRID_WIDTH, GRID_HEIGHT);
    let mut last_tick = Instant::now();

    // Main loop
    loop {
        // Render
        terminal.draw(|frame| render(frame, &game))?;

        // Calculate time until next tick
        let tick_duration = game.tick_duration();
        let timeout = tick_duration
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        // Handle input
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            game.change_direction(Direction::Up);
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            game.change_direction(Direction::Down);
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            game.change_direction(Direction::Left);
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            game.change_direction(Direction::Right);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Update game state
        if last_tick.elapsed() >= tick_duration {
            game.tick();
            last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
