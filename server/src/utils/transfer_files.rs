use std::fs::File;
use std::path::Path;
use std::{
    io::{self, Read, Write},
    net::TcpStream,
};
use std::fs;
use zip::{write::SimpleFileOptions, ZipWriter};

pub fn receive_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::create(path)?;

    // Read file size
    let mut size_buffer = [0; 8];
    stream.read_exact(&mut size_buffer)?;
    let file_size = u64::from_be_bytes(size_buffer);

    // Receive file content
    let mut buffer = [0; 1024];
    let mut total_bytes_received = 0;

    while total_bytes_received < file_size {
        let n = stream.read(&mut buffer)?;
        total_bytes_received += n as u64;
        file.write_all(&buffer[..n])?;
    }

    println!("File received: {}", filename);
    Ok(())
}

pub fn send_file(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = File::open(path)?;

    // Send file size
    let file_size = file.metadata()?.len();
    stream.write_all(&file_size.to_be_bytes())?;
    stream.flush()?;

    // Send file content
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        stream.write_all(&buffer[..n])?;
    }

    println!("File sent: {}", filename);
    Ok(())
}

// Function to send a folder as a zip file
pub fn send_folder(stream: &mut TcpStream, folder_name: &str) -> io::Result<()> {
  let folder_path = Path::new(folder_name);

  if folder_path.is_dir() {
      // Create a zip file in-memory
      let zip_file_name = format!("{}.zip", folder_name);
      let zip_file = File::create(&zip_file_name)?;
      let mut zip = ZipWriter::new(zip_file);

      // Recursively add files to the zip
      zip_dir(&folder_path, &folder_path, &mut zip)?;

      // Finish writing to zip
      zip.finish()?;

      // Send the zip file using the existing send_file function
      send_file(stream, &zip_file_name)?;

      // Optionally, delete the zip file after sending it
      std::fs::remove_file(zip_file_name)?;
  } else {
      println!("Error: {} is not a directory", folder_name);
  }
  Ok(())
}

// Recursively add a directory and its contents to the zip
pub fn zip_dir(dir: &Path, base_dir: &Path, zip: &mut ZipWriter<File>) -> io::Result<()> {
  let dir_entries = fs::read_dir(dir)?;

  for entry in dir_entries {
      let entry = entry?;
      let path = entry.path();

      if path.is_dir() {
          // Recursively zip subdirectories
          zip_dir(&path, base_dir, zip)?;
      } else {
          let name = path.strip_prefix(base_dir).unwrap();
          let mut file = File::open(&path)?;

          zip.start_file(name.to_string_lossy(), SimpleFileOptions::default())?;
          io::copy(&mut file, zip)?;
      }
  }

  Ok(())
}