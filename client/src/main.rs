use std::{
    io::{self, BufRead, Write, Read},
    net::TcpStream,
};

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let mut buffer = [0; 1024];

    loop {
        // Read user input
        let mut input = String::new();
        print!("Enter message: ");
        io::stdout().flush()?;
        reader.read_line(&mut input)?;

        stream.write_all(input.as_bytes())?;

        match stream.read(&mut buffer) {
            Ok(0) => {
                // Connection closed by server
                println!("Server disconnected.");
                break;
            }
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[..n]);
                println!("Server response: {}", message);
            }
            Err(e) => {
                println!("Failed to read from stream: {}", e);
            }
        }
    }
    Ok(())
}
