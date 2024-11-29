use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::{path::Path, time::{Duration, Instant}};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use futures::future::join_all;


#[tokio::main]
async fn main() -> io::Result<()> {
   let args: Vec<String> = env::args().collect();


   // Modified to accept only one argument but maintain multiple server capability
   if args.len() < 3 {
       eprintln!("Usage: client <server_ip:port>");
       return Ok(());
   }


   // Use a single server address for testing but retain a vector for future expansion
   let server_addrs = &args[1..];
   let image_folder_path = "./images"; // Path to the folder with images


   // Create the output directory if it doesn't exist
   let encoded_images_path = Path::new("./encoded_images");
   if !encoded_images_path.exists() {
       fs::create_dir_all(encoded_images_path)?;
   }


   // Collect tasks for sending and receiving images
   let mut tasks = Vec::new();
   let mut total_duration = Duration::new(0, 0);
   let mut count = 0;


   // Loop over all files in the specified directory and start sending them
   for entry in fs::read_dir(image_folder_path)? {
       let entry = entry?;
       let path = entry.path();
       count += 1;


       // Check if the entry is a file and has a valid image extension
       if path.is_file() {
           let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_lowercase();
           if extension == "png" || extension == "jpg" || extension == "jpeg" {
               // For each server address, handle sending and receiving concurrently
               for server_addr in server_addrs {
                   let path_clone = path.clone();
                   let server_addr_clone = server_addr.clone().to_string();
                   // Start the send/receive operation as a single task
                   tasks.push(tokio::spawn(async move {
                       let start_time = Instant::now();


                       // Retry mechanism
                       let max_retries = 5;
                       let mut attempt = 0;
                       loop {
                           if attempt >= max_retries {
                               eprintln!(
                                   "Max retries reached for image {:?} with server {}",
                                   path_clone, server_addr_clone
                               );
                               break;
                           }


                           if let Err(e) = send_and_receive_image(&server_addr_clone, &path_clone, encoded_images_path).await {
                               eprintln!(
                                   "Error processing image {:?} with server {}: {:?}, attempt {}",
                                   path_clone, server_addr_clone, e, attempt + 1
                               );
                               attempt += 1;
                           } else {
                               println!(
                                   "Successfully processed image {:?} with server {}",
                                   path_clone, server_addr_clone
                               );
                               break;
                           }
                       }


                       let duration = start_time.elapsed();
                       duration
                   }));
               }
           }
       }
   }


   // Wait for all tasks to complete and accumulate the durations
   let durations = join_all(tasks).await;
   for result in durations {
       if let Ok(duration) = result {
           total_duration += duration;
       }
   }


   // Calculate average time
   if count > 0 {
       let average_duration = total_duration / count as u32;
       println!("Average round-trip time: {:?}", average_duration);
   } else {
       println!("No images were processed.");
   }


   Ok(())
}


async fn send_and_receive_image(server_addr: &str, image_path: &Path, encoded_images_path: &Path) -> io::Result<()> {
   let file_name = image_path.file_name().and_then(|name| name.to_str()).unwrap_or("image");


   // Read the image file
   let mut file = File::open(image_path)?;
   let mut buffer = Vec::new();
   file.read_to_end(&mut buffer)?;


   // Connect to the server
   let mut socket: TcpStream = TcpStream::connect(server_addr).await?;


   // Send file name
   socket.write_all(file_name.as_bytes()).await?;


   // Wait for acknowledgment from the server
   let mut ack = [0u8; 3];
   socket.read_exact(&mut ack).await?;
   if &ack != b"ACK" {
       eprintln!("Failed to receive acknowledgment from server.");
       return Ok(());
   }


   // Send image data
   socket.write_all(&buffer).await?;
   println!("Sent image {} to {}", file_name, server_addr);


   // Wait to receive the encoded image back from the server
   let encoded_file_path = encoded_images_path.join(create_encoded_filename(image_path, server_addr));
   let mut encoded_file = tokio::fs::File::create(&encoded_file_path).await?;


   // Receive the encoded image
   let mut temp_buffer = [0u8; 1024];
   loop {
       let n = socket.read(&mut temp_buffer).await?;
       if n == 0 {
           break; // Server closed the connection
       }
       encoded_file.write_all(&temp_buffer[..n]).await?;
   }


   println!("Received and saved encoded image as {:?}", encoded_file_path);


   Ok(())
}


fn create_encoded_filename(original_path: &Path, server_addr: &str) -> String {
   let file_name = original_path.file_name().unwrap().to_str().unwrap();
   format!("{}_encoded_{}.png", file_name, server_addr.replace(":", "_"))
}
