use clap::Parser;
use crossterm::{
    event::{self, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::Backend,
    layout::Rect,
    prelude::{CrosstermBackend, Stylize, Terminal},
    widgets::{Paragraph, Wrap},
    Frame,
};
use std::{io::stdout, path::PathBuf};
use tplan::{Error, Task, TodoFile};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set a todo.txt file to open
    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,
}

#[derive(PartialEq, Eq)]
enum Mode {
    View,
    Select(usize),
    Edit {
        item: usize,
        cursor: usize,
        text: String,
    },
}

struct App {
    todo_file: TodoFile,
    mode: Mode,
    pub is_running: bool,
    pub is_dirty: bool,
}

impl App {
    fn load(file_path: PathBuf) -> Result<Self, Error> {
        let todo_file = TodoFile::load(&file_path)?;
        Ok(Self {
            todo_file,
            mode: Mode::View,
            is_running: true,
            is_dirty: false,
        })
    }

    fn quit(&mut self) {
        self.is_running = false;
    }

    fn select_first(&mut self) {
        self.mode = Mode::Select(0);
    }

    fn select_last(&mut self) {
        self.mode = Mode::Select(self.todo_file.tasks.len() - 1);
    }

    fn select_next(&mut self) {
        if let Mode::Select(i) = &mut self.mode {
            *i = (*i + 1).min(self.todo_file.tasks.len() - 1);
        } else {
            self.mode = Mode::Select(0);
        }
    }

    fn select_prev(&mut self) {
        if let Mode::Select(i) = &mut self.mode {
            *i = i.saturating_sub(1);
        } else {
            self.mode = Mode::Select(0);
        }
    }

    fn complete_selected(&mut self) {
        if let Mode::Select(i) = self.mode {
            self.todo_file.tasks[i].completed = !self.todo_file.tasks[i].completed;
            self.is_dirty = true;
        }
    }

    fn delete_selected(&mut self) {
        if let Mode::Select(i) = self.mode {
            self.todo_file.tasks.remove(i);
            self.is_dirty = true;
        }
    }

    fn edit_task(&mut self) {
        let index = if let Mode::Select(i) = self.mode {
            i
        } else {
            0
        };
        let text = self.todo_file.tasks[index].summary.clone();
        self.mode = Mode::Edit {
            item: index,
            cursor: text.len(),
            text,
        };
    }

    fn add_edit_task(&mut self, index: usize) {
        self.todo_file.tasks.insert(index, Task::default());
        self.mode = Mode::Edit {
            item: index,
            text: String::new(),
            cursor: 0,
        };
    }

    fn insert_task(&mut self) {
        let index = if let Mode::Select(i) = self.mode {
            i.saturating_sub(1)
        } else {
            0
        };
        self.add_edit_task(index);
    }

    fn append_task(&mut self) {
        let index = if let Mode::Select(i) = self.mode {
            i + 1
        } else {
            self.todo_file.tasks.len()
        };
        self.add_edit_task(index);
    }

    fn save(&mut self) -> Result<(), Error> {
        self.todo_file.save()?;
        self.is_dirty = false;
        Ok(())
    }

    fn save_edit(&mut self) {
        if let Mode::Edit { item, text, .. } = &self.mode {
            self.todo_file.tasks[*item].summary = text.clone();
            self.mode = Mode::Select(*item);
        }
        self.is_dirty = true;
    }

    fn cancel_edit(&mut self) {
        if let Mode::Edit { item, .. } = &self.mode {
            self.mode = Mode::Select(*item);
        }
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let file_path = cli.file.unwrap_or_else(|| {
        let mut path = dirs::document_dir().unwrap();
        path.push("todo.txt");
        path
    });
    let mut app = App::load(file_path)?;

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    run(&mut terminal, &mut app)?;

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Error> {
    terminal.draw(|f| draw(f, app))?;
    while app.is_running {
        let mut redraw = false;
        if event::poll(std::time::Duration::from_millis(1))? {
            if let event::Event::Key(key) = event::read()? {
                match &mut app.mode {
                    Mode::View => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('a') => {
                            app.append_task();
                            redraw = true;
                        }
                        KeyCode::Char('g') | KeyCode::Home => {
                            app.select_first();
                            redraw = true;
                        }
                        KeyCode::Char('G') | KeyCode::End => {
                            app.select_last();
                            redraw = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next();
                            redraw = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev();
                            redraw = true;
                        }
                        _ => {}
                    },
                    Mode::Select(_) => match key.code {
                        KeyCode::Char('g') | KeyCode::Home => {
                            app.select_first();
                            redraw = true;
                        }
                        KeyCode::Char('G') | KeyCode::End => {
                            app.select_last();
                            redraw = true;
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.select_next();
                            redraw = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.select_prev();
                            redraw = true;
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => {
                            app.complete_selected();
                            redraw = true;
                        }
                        KeyCode::Char('x') | KeyCode::Delete => {
                            app.delete_selected();
                            // TOFIX: Index out of bounds
                            redraw = true;
                        }
                        KeyCode::Char('i') => {
                            app.insert_task();
                            redraw = true;
                        }
                        KeyCode::Char('a') => {
                            app.append_task();
                            redraw = true;
                        }
                        KeyCode::Char('c') | KeyCode::Char('e') => {
                            app.edit_task();
                            redraw = true;
                        }
                        KeyCode::Char('q') => app.quit(),
                        _ => {}
                    },
                    Mode::Edit {
                        item: _,
                        cursor,
                        text,
                    } => match key.code {
                        KeyCode::Esc => {
                            app.cancel_edit();
                            redraw = true;
                        }
                        KeyCode::Char(c) => {
                            text.insert(*cursor, c);
                            *cursor += 1;
                            redraw = true;
                        }
                        KeyCode::Backspace => {
                            if *cursor > 0 {
                                *cursor -= 1;
                                text.remove(*cursor);
                                redraw = true;
                            }
                        }
                        KeyCode::Delete => {
                            if *cursor < text.len() {
                                text.remove(*cursor);
                                redraw = true;
                            }
                        }
                        KeyCode::Left => {
                            if *cursor > 0 {
                                *cursor -= 1;
                                redraw = true;
                            }
                        }
                        KeyCode::Right => {
                            if *cursor < text.len() {
                                *cursor += 1;
                                redraw = true;
                            }
                        }
                        KeyCode::Enter => {
                            app.save_edit();
                            redraw = true;
                        }
                        _ => {}
                    },
                }
            }
        }
        if app.is_dirty {
            app.save()?;
        }
        if redraw {
            terminal.draw(|f| draw(f, app))?;
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.size();
    frame.render_widget(Paragraph::new("TPLAN").blue().bold(), area);
    let mut line_area = Rect { height: 1, ..area };
    line_area.x += 2;
    line_area.width -= 2;
    for (i, task) in app.todo_file.tasks.iter().enumerate() {
        line_area.y += 1;
        if line_area.bottom() > area.bottom() {
            break;
        }
        let paragraph = if task.completed {
            Paragraph::new(task.summary.clone())
                .wrap(Wrap { trim: true })
                .reset()
                .dim()
                .italic()
                .crossed_out()
        } else {
            Paragraph::new(task.summary.clone())
                .wrap(Wrap { trim: true })
                .reset()
                .white()
        };
        if Mode::Select(i) == app.mode {
            frame.render_widget(paragraph.black().on_yellow(), line_area);
            continue;
        } else if let Mode::Edit { item, text, cursor } = &app.mode {
            if *item == i {
                frame.set_cursor(line_area.x + *cursor as u16, line_area.y);
                let par = Paragraph::new(text.clone())
                    .wrap(Wrap { trim: true })
                    .reset();
                frame.render_widget(par.black().on_blue(), line_area);
                continue;
            }
        }
        frame.render_widget(paragraph, line_area);
    }
}
