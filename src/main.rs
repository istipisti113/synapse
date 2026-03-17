mod ui;
mod player;

use std::{io, time::Duration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ui::app::App;
use tokio;
use tokio_util::sync::CancellationToken;

use mpris_server::{
    zbus::{ Result}, Player,};

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let music_path = args.get(1).cloned().unwrap_or_else(|| ".".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mpris_control = Arc::new(Mutex::new(String::from("")));
    let token = CancellationToken::new();

    let player = Player::builder("synapce") //the name by which it can be controlled
        .can_play(true)
        .can_pause(true)
        .can_go_previous(true)
        .can_go_next(true)
        .build()
        .await.unwrap();

    let ctrl = Arc::clone(&mpris_control);
    player.connect_play_pause(move |_player| {
        let mut control = ctrl.lock().unwrap();
        control.push_str("playpause");
    });

    let ctrl = Arc::clone(&mpris_control);
    player.connect_previous(move |_player| {
        let mut control = ctrl.lock().unwrap();
        control.push_str("previous");
    });

    let ctrl = Arc::clone(&mpris_control);
    player.connect_next(move |_player| {
        let mut control = ctrl.lock().unwrap();
        control.push_str("next");
    });

    let ctrl = Arc::clone(&mpris_control);
    player.connect_set_volume(move |_player, volume| {
        let mut control = ctrl.lock().unwrap();
        if volume < 1.0 {
            control.push_str("volume_down");
        } else if volume >1.0{
            control.push_str("volume_up");
        }
    });

    let token_clone = token.clone();
    tokio::task::spawn_blocking( move || {
        let mut app = App::new(music_path);
        loop {
            terminal.draw(|f| ui::render(f, &mut app)).unwrap();
            app.update_time();
            app.check_track_finished();

            let mut ctrl = mpris_control.lock().unwrap();
            match &*ctrl.as_str() {
                "playpause" =>{
                    app.toggle_playback();
                },
                "next"=>{
                    app.next_track();
                },
                "previous"=>{
                    app.previous_track();
                },
                "volume_up"=>{
                    app.volume_up();
                },
                "volume_down"=>{
                    app.volume_down();
                },
                _ => {}
            }
            ctrl.clear();

            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    match key.code {
                        KeyCode::Char('q') => {
                            token_clone.cancel();
                            break;
                        },
                        KeyCode::Char('j') => app.next_song_in_list(),
                        KeyCode::Char('k') => app.previous_song_in_list(),
                        KeyCode::Char('h') => app.seek_backward(),
                        KeyCode::Char('l') => app.seek_forward(),
                        KeyCode::Char(' ') => app.toggle_playback(),
                        KeyCode::Up => app.volume_up(),
                        KeyCode::Down => app.volume_down(),
                        KeyCode::Char('m') => app.toggle_play_mode(),
                        KeyCode::Char('n') => app.next_track(),     // Переход на следующий трек
                        KeyCode::Char('b') => app.previous_track(), // Переход на предыдущий трек
                        KeyCode::Enter => app.play_selected(),
                        _ => {}
                    }
                }
            }
        }


        disable_raw_mode().unwrap();
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    });

    tokio::select! {
        _=player.run()=>{}
        _=token.cancelled()=>{}
    }
    Ok(())
}
