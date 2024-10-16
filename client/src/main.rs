use std::fs::File;
use std::path::Path;
use std::{
    io::{self, BufRead, Read, Write},
    net::TcpStream,
};

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        // Read user input
        let mut input = String::new();
        print!("Enter command: ");
        io::stdout().flush()?;
        reader.read_line(&mut input)?;
        let trimmed_input = input.trim();

        if trimmed_input.starts_with("upload") {
            let filename = trimmed_input
                .split_whitespace()
                .nth(1)
                .expect("No file name provided");
            upload_file(&mut stream, filename)?;
        } else if trimmed_input.starts_with("download") {
            let filename = trimmed_input
                .split_whitespace()
                .nth(1)
                .expect("No file name provided");
            download_file(&mut stream, filename)?;
        } else {
            send_message(&mut stream, trimmed_input)?;
            receive_response(&mut stream)?;
        }
    }
}

fn send_message(stream: &mut TcpStream, message: &str) -> io::Result<()> {
    stream.write_all(message.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn receive_response(stream: &mut TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];

    match stream.read(&mut buffer) {
        Ok(0) => {
            println!("Server disconnected.");
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Server disconnected",
            ));
        }
        Ok(n) => {
            let response = String::from_utf8_lossy(&buffer[..n]);
            println!("\nResponse:\n{}", response);
        }
        Err(e) => {
            println!("Failed to read from stream: {}", e);
        }
    }
    Ok(())
}

fn upload_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::open(path)?;

    // Send the "upload" command with the file name
    let command = format!("upload {}", filename);
    send_message(stream, &command)?;

    // Send the file size
    let file_size = file.metadata()?.len();
    stream.write_all(&file_size.to_be_bytes())?;
    stream.flush()?;

    // Send the file content
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buffer[..n])?;
    }
    println!("File uploaded: {}", filename);
    Ok(())
}

fn download_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    // Send the "download" command with the file name
    let command = format!("download {}", filename);
    send_message(stream, &command)?;

    // Receive the file size
    let mut size_buffer = [0; 8];
    stream.read_exact(&mut size_buffer)?;
    let file_size = u64::from_be_bytes(size_buffer);

    // Receive the file content
    let mut file = File::create(filename)?;
    let mut total_bytes_read = 0;
    let mut buffer = [0; 1024];

    while total_bytes_read < file_size {
        let n = stream.read(&mut buffer)?;
        total_bytes_read += n as u64;
        file.write_all(&buffer[..n])?;
    }

    println!("File downloaded: {}", filename);
    Ok(())
}
