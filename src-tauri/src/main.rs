// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::{CustomMenuItem, Manager, SystemTray, SystemTrayMenu, SystemTrayMenuItem};
use std::sync::Mutex;

use tokio::sync::mpsc;

#[derive(Serialize,Deserialize, Debug)]
enum PomodoroCommand {
    StartTask,
    CancelTask,
}

#[derive(Debug,Deserialize,Serialize)]
enum PomodoroEvent {
    TaskStarted,
    PauseStarted,
    Idle,
    TimerTick(u64),
}

#[derive(Debug,Deserialize,Serialize,Clone)]
enum TimerState{
    Idle,
    Running,
    Paused,
}

#[derive(Debug)]
struct Task{
    name: String,
    start_time: chrono::DateTime<chrono::Local>,
    end_time: Option<chrono::DateTime<chrono::Local>>,
}

impl Task {
    fn new(name: String) -> Self {
        Task {
            name,
            start_time: chrono::offset::Local::now(),
            end_time: None,
        }
    }

    fn finish(&mut self) {
        self.end_time = Some(chrono::offset::Local::now());
    }
}


struct PomodoState{
    timer_state: TimerState,
    past_tasks: Vec<Task>,
    current_task: Option<Task>,
    cmd_channel: mpsc::Sender<PomodoroCommand>,
    event_channel: mpsc::Receiver<PomodoroEvent>,
}

impl PomodoState {
    fn new(cmd_channel: mpsc::Sender<PomodoroCommand>,event_channel: mpsc::Receiver<PomodoroEvent>) -> Self {
        PomodoState {
            timer_state: TimerState::Idle,
            past_tasks: Vec::new(),
            current_task: None,
            cmd_channel,
            event_channel,
        }
    }

    fn start_task(&mut self, task: String) -> Result<(), String> {
        match self.timer_state {
            TimerState::Idle => {
                self.timer_state = TimerState::Running;
                self.current_task = Some(Task::new(task));

                self.cmd_channel.try_send(PomodoroCommand::StartTask)
                    .map_err(|e| e.to_string())
            },
            TimerState::Running => {
                Err("Timer already running".to_string())
            },
            TimerState::Paused => {
                Err("Timer is paused".to_string())
            },
        }
    }

}

type Pomodoro = Mutex<PomodoState>;




// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn start_task(task: &str, pomodoro: tauri::State<'_,Pomodoro>) -> Result<(), String> {
    let mut state = pomodoro.lock().unwrap();
    state.start_task(task.to_string())
}

fn tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide);
    SystemTray::new()
        .with_menu(tray_menu)
}

#[tokio::main]
async fn main() {
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    let (to_timer_tx, to_timer_rx) = mpsc::channel::<PomodoroCommand>(1);
    let (from_timer_tx, from_timer_rx) = mpsc::channel::<PomodoroEvent>(1);
    let state: Pomodoro = Mutex::new(PomodoState::new(to_timer_tx, from_timer_rx));

    tokio::spawn( async move {
        async_timer(to_timer_rx, from_timer_tx).await;
    });


    tauri::Builder::default()
        .manage(state)
        .system_tray(tray())
        .on_system_tray_event(|app, event| match event{
            tauri::SystemTrayEvent::MenuItemClick { id , ..} => {
                match id.as_str(){
                    "quit" => {
                        app.exit(0);
                    },
                    "hide" => {
                        let item_handle = app.tray_handle().get_item(&id);
                        let window = app.get_window("main").unwrap();
                        if window.is_visible().unwrap() {
                            window.hide().unwrap();
                            item_handle.set_title("Show").unwrap();
                        } else {
                            window.show().unwrap();
                            item_handle.set_title("Hide").unwrap();
                        }
                    },
                    _ => {}
                }
            },
            tauri::SystemTrayEvent::LeftClick { ..} => {/*do nothing*/},
            tauri::SystemTrayEvent::RightClick{ .. } => {/*do nothing*/},
            tauri::SystemTrayEvent::DoubleClick {..} => {/*do nothing*/},
            _ => {/*do nothing*/},
        })
        .setup(|app|{
            let handle = app.handle();
            tokio::spawn(async move{
                async_controller(handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_task])
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
              event.window().hide().unwrap();
              event.window().app_handle().tray_handle().get_item("hide").set_title("Show").unwrap();
              api.prevent_close();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

enum ProcessState{
    Idle,
    Running{since: std::time::Instant},
    Paused{since: std::time::Instant},
}
impl ProcessState{
    fn is_ready(&self)-> bool{
        match self {
            ProcessState::Idle => true,
            ProcessState::Running { since } => {
                let diff = since.elapsed().as_secs();
                diff >= 25 * 60 // 25 minutes
            },
            ProcessState::Paused { since } => {
                let diff = since.elapsed().as_secs();
                diff >= 5 * 60 // 5 minutes
            },
        }
    }
}


async fn async_controller(app: tauri::AppHandle){

    loop{
        let state = app.state::<Pomodoro>();
        let mut state = state.lock().unwrap();
        let event = state.event_channel.try_recv();
        match event{
            Ok(PomodoroEvent::TaskStarted) => {
                state.timer_state = TimerState::Running;
                // send notification to UI
                app.emit_all("state-changed", TimerState::Running)
                    .expect("failed to emit event");
            },
            Ok(PomodoroEvent::PauseStarted) => {
                let mut task = state.current_task.take().unwrap();
                task.finish();
                state.past_tasks.push(task);
                state.timer_state = TimerState::Paused;
                // send notification to UI
                app.emit_all("state-changed", TimerState::Paused)
                    .expect("failed to emit event");
                let window = app.get_window("main").unwrap();
                if !window.is_visible().unwrap() {
                    window.show().unwrap();
                    let item_handle = app.tray_handle().get_item("hide");
                    item_handle.set_title("Show").unwrap();
                }
            },
            Ok(PomodoroEvent::Idle) => {
                state.timer_state = TimerState::Idle;
                // send notification to UI
                app.emit_all("state-changed", TimerState::Idle)
                    .expect("failed to emit event");
                let window = app.get_window("main").unwrap();
                if !window.is_visible().unwrap() {
                    window.show().unwrap();
                    let item_handle = app.tray_handle().get_item("hide");
                    item_handle.set_title("Show").unwrap();
                }
            },
            Ok(PomodoroEvent::TimerTick(tick)) => {
                app.emit_all("tick", tick)
                    .expect("failed to emit event");
            },
            Err(_) => {},
        }
    }
}

async fn async_timer(mut rx: mpsc::Receiver<PomodoroCommand>, tx: mpsc::Sender<PomodoroEvent>){

    let mut state = ProcessState::Idle;
    // #Note: is tokio::select! better?
    loop {
        match state {
            ProcessState::Idle => {
                // wait for a received command
                let task = rx.recv().await.unwrap();
                match task{
                    PomodoroCommand::StartTask => {
                        state = ProcessState::Running { since: std::time::Instant::now() };
                        // send notification
                        tx.send(PomodoroEvent::TaskStarted).await.unwrap();
                    },
                    PomodoroCommand::CancelTask => {
                        println!("Cancel task: {:?}. NOT YET IMPLEMENTED!", task);
                    },
                }
            },
            ProcessState::Running{ since } => {
                if state.is_ready(){
                    state = ProcessState::Paused { since: std::time::Instant::now() };
                    // send notification
                    tx.send(PomodoroEvent::PauseStarted).await.unwrap();
                } else {
                    // send notification
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let tick = since.elapsed().as_secs();
                    tx.send(PomodoroEvent::TimerTick(tick)).await.unwrap();
                }
            },
            ProcessState::Paused{ since } => {
                if state.is_ready(){
                    state = ProcessState::Idle;
                    // send notification
                    tx.send(PomodoroEvent::Idle).await.unwrap();
                } else {
                    // send notification
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let tick = since.elapsed().as_secs();
                    tx.send(PomodoroEvent::TimerTick(tick)).await.unwrap();
                }
            },
        }


    }
}
