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
    fs,
    io::{self, stdout},
    path::Path,
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

// High score file
const HIGH_SCORE_FILE: &str = "highestscore.txt";

// ============================================================================
// Types
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i16,
    y: i16,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

// ============================================================================
// High Score Persistence
// ============================================================================

fn load_high_score(path: &Path) -> u32 {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn save_high_score(path: &Path, score: u32) {
    let _ = fs::write(path, score.to_string());
}

struct Game {
    snake: VecDeque<Position>,
    direction: Direction,
    next_direction: Direction,
    food: Position,
    score: u32,
    high_score: u32,
    state: GameState,
    grid_width: u16,
    grid_height: u16,
}

// ============================================================================
// Game Logic
// ============================================================================

impl Game {
    fn new(grid_width: u16, grid_height: u16, high_score: u32) -> Self {
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
            high_score,
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
            self.end_game();
            return;
        }

        // Check self collision
        if self.snake.contains(&new_head) {
            self.end_game();
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

    fn restart(&mut self) {
        let center_x = self.grid_width as i16 / 2;
        let center_y = self.grid_height as i16 / 2;

        // Reset snake to initial position
        self.snake.clear();
        for i in 0..INITIAL_SNAKE_LENGTH {
            self.snake.push_back(Position {
                x: center_x - i as i16,
                y: center_y,
            });
        }

        // Reset game state
        self.direction = Direction::Right;
        self.next_direction = Direction::Right;
        self.score = 0;
        self.state = GameState::Playing;

        // Spawn new food
        self.spawn_food();
    }

    fn is_game_over(&self) -> bool {
        matches!(self.state, GameState::GameOver)
    }

    fn end_game(&mut self) {
        if self.score > self.high_score {
            self.high_score = self.score;
        }
        self.state = GameState::GameOver;
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
        Line::from(vec![
            Span::styled("Highest Score: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", game.high_score), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("Your Score: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", game.score), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press R to restart | ESC to quit",
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

    let popup_area = centered_rect(36, 11, area);
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

    // Load high score and create game
    let high_score_path = Path::new(HIGH_SCORE_FILE);
    let high_score = load_high_score(high_score_path);
    let mut game = Game::new(GRID_WIDTH, GRID_HEIGHT, high_score);
    let mut last_tick = Instant::now();
    let mut was_playing = true;

    // Main loop
    loop {
        // Render
        terminal.draw(|frame| render(frame, &game))?;

        // Save high score when game ends
        if was_playing && game.is_game_over() {
            save_high_score(high_score_path, game.high_score);
            was_playing = false;
        }

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
                        KeyCode::Char('r') | KeyCode::Char('R') if game.is_game_over() => {
                            game.restart();
                            last_tick = Instant::now();
                            was_playing = true;
                        }
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restart_resets_score() {
        let mut game = Game::new(10, 10, 0);
        game.score = 5;
        game.state = GameState::GameOver;

        game.restart();

        assert_eq!(game.score, 0);
    }

    #[test]
    fn test_restart_resets_state_to_playing() {
        let mut game = Game::new(10, 10, 0);
        game.state = GameState::GameOver;

        game.restart();

        assert!(matches!(game.state, GameState::Playing));
    }

    #[test]
    fn test_restart_resets_snake_length() {
        let mut game = Game::new(10, 10, 0);
        // Simulate snake growth
        game.snake.push_front(Position { x: 0, y: 0 });
        game.snake.push_front(Position { x: 1, y: 0 });
        game.state = GameState::GameOver;

        game.restart();

        assert_eq!(game.snake.len(), INITIAL_SNAKE_LENGTH);
    }

    #[test]
    fn test_restart_resets_snake_position_to_center() {
        let mut game = Game::new(10, 10, 0);
        game.state = GameState::GameOver;

        game.restart();

        let head = game.snake.front().unwrap();
        assert_eq!(head.x, 5); // center of 10-width grid
        assert_eq!(head.y, 5); // center of 10-height grid
    }

    #[test]
    fn test_restart_resets_direction() {
        let mut game = Game::new(10, 10, 0);
        game.direction = Direction::Up;
        game.next_direction = Direction::Left;
        game.state = GameState::GameOver;

        game.restart();

        assert_eq!(game.direction, Direction::Right);
        assert_eq!(game.next_direction, Direction::Right);
    }

    #[test]
    fn test_is_game_over_returns_true_when_game_over() {
        let mut game = Game::new(10, 10, 0);
        game.state = GameState::GameOver;

        assert!(game.is_game_over());
    }

    #[test]
    fn test_is_game_over_returns_false_when_playing() {
        let game = Game::new(10, 10, 0);

        assert!(!game.is_game_over());
    }

    #[test]
    fn test_restart_spawns_food_on_grid() {
        let mut game = Game::new(10, 10, 0);
        game.state = GameState::GameOver;

        game.restart();

        assert!(game.food.x >= 0 && game.food.x < 10);
        assert!(game.food.y >= 0 && game.food.y < 10);
    }

    #[test]
    fn test_restart_food_not_on_snake() {
        let mut game = Game::new(10, 10, 0);
        game.state = GameState::GameOver;

        game.restart();

        assert!(!game.snake.contains(&game.food));
    }

    #[test]
    fn test_game_initializes_with_high_score() {
        let game = Game::new(10, 10, 42);

        assert_eq!(game.high_score, 42);
    }

    #[test]
    fn test_end_game_updates_high_score_when_score_is_higher() {
        let mut game = Game::new(10, 10, 5);
        game.score = 10;

        game.end_game();

        assert_eq!(game.high_score, 10);
        assert!(game.is_game_over());
    }

    #[test]
    fn test_end_game_keeps_high_score_when_score_is_lower() {
        let mut game = Game::new(10, 10, 10);
        game.score = 5;

        game.end_game();

        assert_eq!(game.high_score, 10);
        assert!(game.is_game_over());
    }

    #[test]
    fn test_end_game_keeps_high_score_when_score_is_equal() {
        let mut game = Game::new(10, 10, 5);
        game.score = 5;

        game.end_game();

        assert_eq!(game.high_score, 5);
        assert!(game.is_game_over());
    }

    #[test]
    fn test_restart_preserves_high_score() {
        let mut game = Game::new(10, 10, 10);
        game.score = 15;
        game.end_game();

        game.restart();

        assert_eq!(game.high_score, 15);
        assert_eq!(game.score, 0);
    }

    #[test]
    fn test_load_high_score_returns_zero_for_missing_file() {
        let path = Path::new("/nonexistent/path/highestscore.txt");

        assert_eq!(load_high_score(path), 0);
    }

    #[test]
    fn test_load_high_score_returns_zero_for_invalid_content() {
        use std::io::Write;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        write!(temp_file, "not_a_number").unwrap();

        assert_eq!(load_high_score(temp_file.path()), 0);
    }

    #[test]
    fn test_save_and_load_high_score() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();

        save_high_score(path, 42);
        let loaded = load_high_score(path);

        assert_eq!(loaded, 42);
    }
}
