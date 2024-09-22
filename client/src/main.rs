use std::{
    io::{self, BufRead, Write, Read},
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
        print!("Enter message: ");
        io::stdout().flush()?;
        reader.read_line(&mut input)?;

        send_message(&mut stream, &input)?;

        receive_response(&mut stream)?;
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
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Server disconnected"));
        }
        Ok(n) => {
            let response = String::from_utf8_lossy(&buffer[..n]);
            println!("Server response:\n{}", response);
        }
        Err(e) => {
            println!("Failed to read from stream: {}", e);
        }
    }
    Ok(())
}