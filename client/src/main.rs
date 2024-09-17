use std::{io::{self, Write}, net::TcpStream};

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let mut stream = TcpStream::connect("127.0.0.1:8080")?;

    stream.write_all(b"Hello, server!")?;

    Ok(())
}
