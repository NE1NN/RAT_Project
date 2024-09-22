use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
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
    if message.trim() == "ls" {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(&["/C", "dir"]).output()?
        } else {
            Command::new("ls").output()?
        };
        println!(
            "Command output: {}",
            String::from_utf8_lossy(&output.stdout)
        );

        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    return Ok(format!("Unknown command: {}\n", message.trim()));
}
