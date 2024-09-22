use std::env;
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = process_stream(stream) {
                    eprintln!("Failed to process {}", e)
                }
            }
            Err(e) => {
                eprintln!("Failed to process {}", e)
            }
        }
    }

    Ok(())
}

fn process_stream(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1204];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed by client
                println!("Client disconnected.");
                break;
            }
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[..n]);
                let response = execute_command(&message)?;

                stream.write_all(response.as_bytes())?;
                stream.flush()?;
            }
            Err(e) => {
                println!("Failed to read from stream: {}", e);
            }
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
                Ok(format!(
                    "Command '{}' executed successfully.\n",
                    command
                ))
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
