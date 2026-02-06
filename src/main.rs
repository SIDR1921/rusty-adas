use std::io;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection};
use rand::Rng;

use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, List, ListItem},
    layout::{Layout, Constraint, Direction},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

struct AppState {
    logs: Vec<String>,
    sensor_status: Vec<(u32, String)>,
}

impl AppState {
    fn new() -> Self {
        Self {
            logs: Vec::new(),
            sensor_status: vec![
                (0x186A, "Initializing...".to_string()),
                (0x2901, "Initializing...".to_string()),
                (0x186B, "Initializing...".to_string()),
                (0x2902, "Initializing...".to_string()),
            ],
        }
    }

    fn add_log(&mut self, msg: String) {
        self.logs.push(msg);
        if self.logs.len() > 20 {
            self.logs.remove(0);
        }
    }

    fn update_sensor(&mut self, id: u32, status: String) {
        if let Some(s) = self.sensor_status.iter_mut().find(|(sid, _)| *sid == id) {
            s.1 = status;
        }
    }
}

trait SentinelComponent: Send + Sync {
    fn check_status(&self) -> String;
    fn get_id(&self) -> u32;
}

struct BMS_ECU {
    can_id: u32,
    history: Mutex<Vec<f64>>,
}

impl BMS_ECU {
    fn detect_thermal_runaway(&self, cell_voltage: f64) -> bool {
        let mut data = self.history.lock().unwrap();
        if data.len() >= 10 { data.remove(0); }
        data.push(cell_voltage);
        
        if data.len() < 5 { return false; }

        let sum: f64 = data.iter().sum();
        let mean = sum / data.len() as f64;
        let variance: f64 = data.iter().map(|v| (mean - *v).powi(2)).sum::<f64>() / data.len() as f64;
        let std_dev = variance.sqrt();

        std_dev > 0.05 && (cell_voltage - mean).abs() > (2.0 * std_dev)
    }
}

impl SentinelComponent for BMS_ECU {
    fn check_status(&self) -> String {
        let mut rng = rand::thread_rng();
        let voltage: f64 = if rng.gen_bool(0.1) { 2.5 } else { rng.gen_range(3.7..4.1) };

        if self.detect_thermal_runaway(voltage) {
            format!("DTC P0A80: Cell Imbalance Detected! ({:.2}V)", voltage)
        } else {
            format!("Cell Voltage: {:.2}V (Optimal)", voltage)
        }
    }
    fn get_id(&self) -> u32 { self.can_id }
}

struct ADAS_Computer {
    can_id: u32,
    module_name: String,
}

impl SentinelComponent for ADAS_Computer {
    fn check_status(&self) -> String {
        let mut rng = rand::thread_rng();
        
        if rng.gen_bool(0.1) {
            "DTC C1A67: Sensor Blind / Occluded".to_string()
        } else {
            let confidence = rng.gen_range(95..100);
            format!("Tracking [{}]: Confidence {}%", self.module_name, confidence)
        }
    }
    fn get_id(&self) -> u32 { self.can_id }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open("blackbox.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sensor_logs (id INTEGER PRIMARY KEY, sensor_id INTEGER, message TEXT, timestamp TEXT DEFAULT CURRENT_TIMESTAMP)",
        [],
    )?;
    let db_lock = Arc::new(Mutex::new(conn));

    let app_state = Arc::new(Mutex::new(AppState::new()));

    let sensors: Vec<Box<dyn SentinelComponent>> = vec![
        Box::new(BMS_ECU { can_id: 0x186A, history: Mutex::new(Vec::new()) }), 
        Box::new(ADAS_Computer { can_id: 0x2901, module_name: "Front_Radar".to_string() }),
        Box::new(BMS_ECU { can_id: 0x186B, history: Mutex::new(Vec::new()) }),
        Box::new(ADAS_Computer { can_id: 0x2902, module_name: "Lane_Cam".to_string() }),
    ];
    let shared_sensors = Arc::new(sensors);

    for i in 0..shared_sensors.len() {
        let sensor_ref = Arc::clone(&shared_sensors);
        let app_ref = Arc::clone(&app_state);
        let db_ref = Arc::clone(&db_lock);

        thread::spawn(move || {
            loop {
                let sensor = &sensor_ref[i];
                thread::sleep(Duration::from_millis(rand::thread_rng().gen_range(500..1500)));

                let status = sensor.check_status();
                let id = sensor.get_id();

                {
                    let mut app = app_ref.lock().unwrap();
                    app.update_sensor(id, status.clone());
                    
                    if status.contains("DTC") {
                         app.add_log(format!("[CAN ID {:#X}] {}", id, status));
                    }
                }

                let conn = db_ref.lock().unwrap();
                conn.execute("INSERT INTO sensor_logs (sensor_id, message) VALUES (?1, ?2)", params![id, status]).unwrap();
            }
        });
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let app = app_state.lock().unwrap();

            let status_items: Vec<ListItem> = app.sensor_status.iter()
                .map(|(id, msg)| ListItem::new(format!("CAN ID {:#X}: {}", id, msg)))
                .collect();
            
            let status_list = List::new(status_items)
                .block(Block::default().borders(Borders::ALL).title("ECU Network Status (CAN Bus)"));
            f.render_widget(status_list, chunks[0]);

            let log_items: Vec<ListItem> = app.logs.iter()
                .map(|msg| ListItem::new(msg.clone()))
                .collect();
            
            let log_list = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title("OBD-II Diagnostic Trouble Codes (DTC)"));
            f.render_widget(log_list, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}