use std::{
    io::{Error, Read},
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    for stream in listener.incoming() {
        process_stream(stream);
    }

    Ok(())
}

fn process_stream(stream: Result<TcpStream, Error>) {
    let mut stream = stream.unwrap();
    let mut buffer = [0; 0124];

    match stream.read(&mut buffer) {
        Ok(_) => {
            let message = String::from_utf8_lossy(&buffer[..]);
            println!("Received: {}", message)
        }
        Err(e) => {
            println!("Failed to read from stream: {}", e);
        }
    }
}
