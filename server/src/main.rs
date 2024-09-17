use std::{
    io::{self, Error, Read},
    net::{TcpListener, TcpStream},
    process::Command,
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_) => {
                if let Err(e) = execute_command() {
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

fn process_stream(stream: Result<TcpStream, Error>) -> io::Result<()> {
    let mut stream = stream?;
    let mut buffer = [0; 1204];

    match stream.read(&mut buffer) {
        Ok(_) => {
            let message = String::from_utf8_lossy(&buffer[..]);
            println!("Received: {}", message)
        }
        Err(e) => {
            println!("Failed to read from stream: {}", e);
        }
    }
    Ok(())
}

fn execute_command() -> io::Result<()> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", "dir"]).output()?
    } else {
        Command::new("ls").output()?
    };

    println!(
        "Command output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
