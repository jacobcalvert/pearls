use std::fmt::Write as FmtWrite;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use chrono::Local;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use eyre::Result;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use serde::Serialize;

use crate::db::tasks;

const MONITOR_STATES: [&str; 4] = ["ready", "blocked", "in_progress", "closed"];

const MONITOR_WEB_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Pearls Monitor</title>
  <style>
    :root {
      --base03: #002b36;
      --base02: #073642;
      --base01: #586e75;
      --base00: #657b83;
      --base0: #839496;
      --base1: #93a1a1;
      --base2: #eee8d5;
      --base3: #fdf6e3;
      --yellow: #b58900;
      --orange: #cb4b16;
      --red: #dc322f;
      --magenta: #d33682;
      --violet: #6c71c0;
      --blue: #268bd2;
      --cyan: #2aa198;
      --green: #859900;

      --page-bg: var(--base03);
      --panel-bg: var(--base02);
      --panel-border: var(--base01);
      --text: var(--base0);
      --muted-text: var(--base1);
      --heading: var(--base2);
      --header-bg: var(--base02);
      --task-bg: var(--base00);
      --task-title: var(--base3);
      --task-meta: var(--base1);
      --assignee: #fff;
      --ready: var(--green);
      --blocked: var(--red);
      --in_progress: var(--yellow);
      --closed: var(--blue);
      --button-bg: var(--base01);
      --button-text: var(--base3);
    }

    body.light {
      --page-bg: var(--base3);
      --panel-bg: var(--base2);
      --panel-border: #93a1a1;
      --text: var(--base00);
      --muted-text: var(--base01);
      --heading: var(--base02);
      --header-bg: #93a1a1;
      --task-bg: var(--base2);
      --task-title: var(--base03);
      --task-meta: var(--base01);
      --assignee: var(--base03);
      --button-bg: #93a1a1;
      --button-text: #002b36;
    }

    * {
      box-sizing: border-box;
    }

    html,
    body {
      margin: 0;
      padding: 0;
      min-height: 100vh;
      background: var(--page-bg);
      color: var(--text);
      font-family: "Fira Code", "Cascadia Code", "JetBrains Mono", Consolas, "Courier New", monospace;
    }

    .layout {
      max-width: 1200px;
      margin: 0 auto;
      padding: 16px;
    }

    .header {
      background: var(--header-bg);
      border: 1px solid var(--panel-border);
      padding: 12px 14px;
      margin-bottom: 12px;
      border-radius: 6px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      flex-wrap: wrap;
    }

    .header h1 {
      margin: 0;
      font-size: 1.2rem;
      color: var(--heading);
    }

    .meta {
      color: var(--muted-text);
      margin: 0;
    }

    button {
      background: var(--button-bg);
      color: var(--button-text);
      border: 1px solid var(--panel-border);
      border-radius: 6px;
      cursor: pointer;
      padding: 6px 10px;
    }

    .board {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      align-items: start;
    }

    .column {
      background: var(--panel-bg);
      border: 1px solid var(--panel-border);
      border-radius: 6px;
      min-height: 70vh;
      padding: 10px;
    }

    .column h2 {
      margin: 0 0 8px 0;
      color: var(--heading);
      font-size: 1rem;
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      gap: 8px;
    }

    .state-ready { color: var(--ready); }
    .state-blocked { color: var(--blocked); }
    .state-in_progress { color: var(--in_progress); }
    .state-closed { color: var(--closed); }

    .task {
      background: var(--task-bg);
      border-left: 4px solid var(--ready);
      border-radius: 4px;
      padding: 8px;
      margin: 6px 0;
      display: grid;
      gap: 6px;
      min-width: 0;
    }

    .task.ready { border-left-color: var(--ready); }
    .task.blocked { border-left-color: var(--blocked); }
    .task.in_progress { border-left-color: var(--in_progress); }
    .task.closed { border-left-color: var(--closed); }

    .task .title {
      color: var(--task-title);
      font-size: 0.95rem;
      white-space: normal;
      overflow-wrap: anywhere;
    }

    .task .meta {
      color: var(--task-meta);
      font-size: 0.82rem;
      white-space: normal;
      overflow-wrap: anywhere;
      word-break: break-word;
    }

    .task .assignee {
      color: var(--assignee);
    }

    .empty {
      color: var(--muted-text);
      font-style: italic;
      font-size: 0.85rem;
      padding: 4px 0;
    }

    .error {
      color: var(--orange);
      margin-top: 8px;
    }

    .task {
      cursor: pointer;
    }

    .task:hover {
      filter: brightness(1.06);
    }

    .task-modal {
      position: fixed;
      inset: 0;
      display: none;
      align-items: center;
      justify-content: center;
      padding: 20px;
      background: rgba(0, 0, 0, 0.55);
      z-index: 10;
    }

    .task-modal.visible {
      display: flex;
    }

    .task-modal-content {
      width: min(760px, 100%);
      max-height: 90vh;
      overflow: auto;
      background: var(--panel-bg);
      color: var(--text);
      border: 1px solid var(--panel-border);
      border-radius: 8px;
      box-shadow: 0 20px 60px rgba(0, 0, 0, 0.35);
    }

    .task-modal-header {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      padding: 12px 14px;
      border-bottom: 1px solid var(--panel-border);
      align-items: center;
    }

    .task-modal-header h2 {
      margin: 0;
      color: var(--heading);
      font-size: 1rem;
    }

    .task-modal-body {
      padding: 12px 14px;
      display: grid;
      gap: 8px;
    }

    .task-modal-body .row {
      display: grid;
      gap: 2px;
    }

    .task-modal-body .label {
      color: var(--muted-text);
      font-size: 0.75rem;
    }

    .task-modal-body .value {
      color: var(--text);
      white-space: pre-wrap;
      overflow-wrap: anywhere;
      word-break: break-word;
      line-height: 1.35;
    }

    @media (max-width: 960px) {
      .board {
        grid-template-columns: 1fr 1fr;
      }
    }

    @media (max-width: 640px) {
      .board {
        grid-template-columns: 1fr;
      }
    }
  </style>
</head>
<body>
    <div class="layout">
    <div class="header">
      <div>
        <h1>Pearls Monitor</h1>
        <p class="meta" id="status-line">starting…</p>
      </div>
      <button id="theme-toggle" type="button">Switch to light mode</button>
    </div>
    <p class="meta" id="updated-line">never refreshed</p>
    <div id="board" class="board"></div>
    <p class="error" id="error-line"></p>
  </div>

  <div id="task-modal" class="task-modal" aria-hidden="true" role="dialog" aria-modal="true">
    <div class="task-modal-content">
      <div class="task-modal-header">
        <h2>Task details</h2>
        <button id="task-modal-close" type="button">Close</button>
      </div>
      <div id="task-modal-body" class="task-modal-body"></div>
    </div>
  </div>

  <script>
    const STATES = ["ready", "blocked", "in_progress", "closed"];
    const LABELS = {
      ready: "ready",
      blocked: "blocked",
      in_progress: "in progress",
      closed: "closed",
    };
    const toggle = document.getElementById("theme-toggle");
    const statusLine = document.getElementById("status-line");
    const updatedLine = document.getElementById("updated-line");
    const errorLine = document.getElementById("error-line");
    const board = document.getElementById("board");
    const taskModal = document.getElementById("task-modal");
    const taskModalBody = document.getElementById("task-modal-body");
    const taskModalClose = document.getElementById("task-modal-close");
    const LIGHT_MODE = "light";
    const DARK_MODE = "dark";

    let mode = localStorage.getItem("pearls-monitor-theme") || DARK_MODE;
    if (mode !== DARK_MODE && mode !== LIGHT_MODE) {
      mode = DARK_MODE;
    }

    function applyTheme() {
      if (mode === LIGHT_MODE) {
        document.body.classList.add("light");
        toggle.textContent = "Switch to dark mode";
      } else {
        document.body.classList.remove("light");
        toggle.textContent = "Switch to light mode";
      }
    }

    function renderField(label, value) {
      const row = document.createElement("div");
      row.className = "row";

      const key = document.createElement("div");
      key.className = "label";
      key.textContent = label;

      const val = document.createElement("div");
      val.className = "value";
      val.textContent = value;

      row.appendChild(key);
      row.appendChild(val);
      return row;
    }

    function formatTaskList(values) {
      if (!values || values.length === 0) {
        return "None";
      }
      return values.map((value) => `#${value}`).join(", ");
    }

    function openTaskModal(task, state) {
      taskModalBody.innerHTML = "";

      const id = task.id != null ? `#${task.id}` : "#?";
      const desc = task.desc || "(no description)";
      const assignee = task.assignee || "no assignee";
      const parents = formatTaskList(task.parents);
      const children = formatTaskList(task.children);

      taskModalBody.appendChild(renderField("ID", id));
      taskModalBody.appendChild(renderField("State", state || task.state || "unknown"));
      taskModalBody.appendChild(renderField("Priority", String(task.priority)));
      taskModalBody.appendChild(renderField("Assignee", assignee));
      taskModalBody.appendChild(renderField("Title", task.title || "(no title)"));
      taskModalBody.appendChild(renderField("Description", desc));
      taskModalBody.appendChild(renderField("Parents", parents));
      taskModalBody.appendChild(renderField("Children", children));

      taskModal.classList.add("visible");
      taskModal.setAttribute("aria-hidden", "false");
    }

    function closeTaskModal() {
      taskModal.classList.remove("visible");
      taskModal.setAttribute("aria-hidden", "true");
    }

    function renderColumn(state, tasks) {
      const column = document.createElement("section");
      column.className = "column";
      const heading = document.createElement("h2");
      const title = document.createElement("span");
      title.className = `state-${state}`;
      title.textContent = LABELS[state] || state;
      const counter = document.createElement("span");
      counter.textContent = String(tasks.length);
      counter.className = "meta";
      heading.appendChild(title);
      heading.appendChild(counter);
      column.appendChild(heading);

      if (tasks.length === 0) {
        const empty = document.createElement("p");
        empty.className = "empty";
        empty.textContent = "No tasks";
        column.appendChild(empty);
        return column;
      }

      for (const task of tasks) {
        const card = document.createElement("article");
        card.className = `task ${state}`;
        card.tabIndex = 0;

        const title = document.createElement("div");
        const id = task.id != null ? `#${task.id}` : "#?";
        const titleValue = task.title || "";
        const desc = task.desc || "";
        title.className = "title";
        title.textContent = `${id} p${task.priority} ${titleValue}`.trim();

        const description = document.createElement("div");
        description.className = "meta";
        description.textContent = desc || "(no description)";

        const assignee = document.createElement("div");
        assignee.className = "assignee meta";
        const assigneeValue = task.assignee || "no assignee";
        assignee.textContent = `assignee: ${assigneeValue}`;

        card.appendChild(title);
        card.appendChild(description);
        card.appendChild(assignee);
        card.addEventListener("click", () => openTaskModal(task, state));
        card.addEventListener("keypress", (event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            openTaskModal(task, state);
          }
        });
        column.appendChild(card);
      }

      return column;
    }

    function renderBoard(payload) {
      board.replaceChildren();
      for (const state of STATES) {
        const column = payload.columns.find((col) => col.state === state);
        const tasks = (column && column.tasks) || [];
        board.appendChild(renderColumn(state, tasks));
      }
      const refreshedAt = new Date(payload.refreshed_at);
      const text = Number.isNaN(refreshedAt.getTime())
        ? payload.refreshed_at
        : refreshedAt.toLocaleString();
      updatedLine.textContent = `refreshed: ${text}`;
      statusLine.textContent = payload.status;
      errorLine.textContent = "";
    }

    async function refreshBoard() {
      try {
        const response = await fetch("/api/board");
        if (!response.ok) {
          throw new Error(`request failed: ${response.status}`);
        }
        const payload = await response.json();
        renderBoard(payload);
      } catch (err) {
        errorLine.textContent = `Failed to refresh board: ${err.message || err}`;
      }
    }

    toggle.addEventListener("click", () => {
      mode = mode === DARK_MODE ? LIGHT_MODE : DARK_MODE;
      localStorage.setItem("pearls-monitor-theme", mode);
      applyTheme();
    });
    taskModalClose.addEventListener("click", closeTaskModal);
    taskModal.addEventListener("click", (event) => {
      if (event.target === taskModal) {
        closeTaskModal();
      }
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && taskModal.classList.contains("visible")) {
        closeTaskModal();
      }
    });

    applyTheme();
    refreshBoard();
    setInterval(refreshBoard, 3000);
  </script>
</body>
</html>
"##;

#[derive(Clone)]
struct WebMonitorState {
    conn: sea_orm_migration::sea_orm::DatabaseConnection,
}

#[derive(Debug)]
struct BoardState {
    columns: [Vec<tasks::TaskRow>; 4],
    status: String,
    refreshed_at: String,
}

#[derive(Serialize)]
struct WebBoardColumn {
    state: &'static str,
    count: usize,
    tasks: Vec<tasks::TaskRow>,
}

#[derive(Serialize)]
struct WebBoardPayload {
    refreshed_at: String,
    status: String,
    columns: Vec<WebBoardColumn>,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            columns: Default::default(),
            status: "waiting for first refresh...".to_string(),
            refreshed_at: "never".to_string(),
        }
    }
}

impl BoardState {
    fn state_index(state: &str) -> Option<usize> {
        match state {
            "ready" => Some(0),
            "blocked" => Some(1),
            "in_progress" => Some(2),
            "closed" => Some(3),
            _ => None,
        }
    }

    fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    async fn refresh(
        &mut self,
        conn: &sea_orm_migration::sea_orm::DatabaseConnection,
    ) -> Result<()> {
        let columns = load_columns(conn).await?;

        self.columns = columns;
        self.refreshed_at = now_string();
        self.status = format!("refreshed {} task(s)", self.total_count());
        Ok(())
    }

    fn total_count(&self) -> usize {
        self.columns.iter().map(std::vec::Vec::len).sum()
    }
}

async fn load_columns(
    conn: &sea_orm_migration::sea_orm::DatabaseConnection,
) -> Result<[Vec<tasks::TaskRow>; 4]> {
    let tasks = tasks::list_tasks(conn, &[]).await?;
    let mut grouped: [Vec<tasks::TaskRow>; 4] = Default::default();
    for task in tasks {
        if let Some(index) = BoardState::state_index(&task.state) {
            grouped[index].push(task);
        }
    }

    for column in &mut grouped {
        column.sort_by_key(|task| (task.priority, task.id));
    }

    Ok(grouped)
}

async fn build_board_payload(
    conn: &sea_orm_migration::sea_orm::DatabaseConnection,
) -> Result<WebBoardPayload> {
    let columns = load_columns(conn).await?;
    let columns = columns
        .into_iter()
        .enumerate()
        .map(|(index, tasks)| WebBoardColumn {
            state: MONITOR_STATES[index],
            count: tasks.len(),
            tasks,
        })
        .collect::<Vec<_>>();
    let total_count = columns.iter().map(|column| column.count).sum::<usize>();

    Ok(WebBoardPayload {
        refreshed_at: now_string(),
        status: format!("refreshed {total_count} task(s)"),
        columns,
    })
}

fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S %:z").to_string()
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn cleanup_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> std::result::Result<(), io::Error> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn state_color(state: &str) -> Color {
    match state {
        "ready" => Color::Green,
        "blocked" => Color::Red,
        "in_progress" => Color::Yellow,
        "closed" => Color::Blue,
        _ => Color::White,
    }
}

fn render_column_title(state: &str, count: usize) -> String {
    let mut title = String::new();
    let _ = write!(title, "{state} ({count})");
    title
}

fn task_line(task: &tasks::TaskRow) -> ListItem<'_> {
    let title = task.title.as_deref().unwrap_or("");
    let desc = task.desc.as_deref().unwrap_or("");
    let assignee = task.assignee.as_deref().unwrap_or("no assignee");
    let line = Line::from(vec![
        Span::styled(
            format!("#{} ", task.id),
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(
            format!("p{} ", task.priority),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(title),
        Span::raw(" — "),
        Span::raw(desc),
        Span::raw(format!(" [assignee: {assignee}]")),
    ]);
    ListItem::new(line)
}

fn render_board(f: &mut Frame, state: &BoardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(5),
        ])
        .split(f.area());

    let header = Paragraph::new("Pearls Monitor: TUI (press q or Esc to quit)")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[1]);

    for (index, state_name) in MONITOR_STATES.iter().enumerate() {
        let tasks = &state.columns[index];
        let title = render_column_title(state_name, tasks.len());
        let color = state_color(state_name);
        let mut task_lines = Vec::with_capacity(tasks.len());
        for task in tasks {
            task_lines.push(task_line(task));
        }
        if task_lines.is_empty() {
            task_lines.push(ListItem::new(Line::from(Span::styled(
                "No tasks",
                Style::default().fg(Color::DarkGray),
            ))));
        }
        let list = List::new(task_lines).block(Block::default().borders(Borders::ALL).title(
            Span::styled(
                title,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ));
        f.render_widget(list, columns[index]);
    }

    let footer_lines = vec![
        Line::from(format!("refreshed: {}", state.refreshed_at)),
        Line::from("press q to quit"),
        Line::from(format!("status: {}", state.status)),
    ];
    let footer = Paragraph::new(footer_lines).block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

async fn index_page() -> Html<&'static str> {
    Html(MONITOR_WEB_HTML)
}

async fn web_board_handler(State(state): State<WebMonitorState>) -> impl IntoResponse {
    match build_board_payload(&state.conn).await {
        Ok(payload) => Json(payload).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to load tasks: {err}"),
        )
            .into_response(),
    }
}

pub async fn run_tui(
    conn: sea_orm_migration::sea_orm::DatabaseConnection,
    refresh_interval: Duration,
) -> Result<()> {
    let refresh_interval = if refresh_interval.is_zero() {
        Duration::from_secs(1)
    } else {
        refresh_interval
    };

    let mut terminal = setup_terminal()?;
    let mut state = BoardState::default();
    let mut last_refresh = Instant::now() - refresh_interval;
    let run = async {
        loop {
            if last_refresh.elapsed() >= refresh_interval {
                match state.refresh(&conn).await {
                    Ok(()) => {}
                    Err(err) => state.set_status(format!("refresh failed: {err}")),
                }
                last_refresh = Instant::now();
            }

            terminal.draw(|frame| {
                render_board(frame, &state);
            })?;

            if event::poll(Duration::from_millis(200))?
                && let Event::Key(event) = event::read()?
            {
                match event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        state.set_status("manual refresh requested".to_string());
                        last_refresh = Instant::now() - refresh_interval;
                    }
                    _ => {}
                }
            }

            tokio::time::sleep(Duration::from_millis(16)).await;
        }
        Ok::<(), eyre::Report>(())
    }
    .await;

    let status = run;
    cleanup_terminal(&mut terminal)?;
    status
}

pub async fn run_web(
    conn: sea_orm_migration::sea_orm::DatabaseConnection,
    host: String,
    port: u16,
) -> Result<()> {
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let app = Router::new()
        .route("/", get(index_page))
        .route("/api/board", get(web_board_handler))
        .with_state(WebMonitorState { conn });

    println!("web monitor started at http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
