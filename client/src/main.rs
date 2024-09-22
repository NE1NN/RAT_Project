use std::{io::{self, BufRead, Write}, net::TcpStream};

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

        stream.write_all(input.as_bytes())?;
    }
}
