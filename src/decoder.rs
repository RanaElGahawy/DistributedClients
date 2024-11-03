use steganography::decoder;
use std::fs::File;
use std::io::Write;
use std::env;
use std::path::Path;

fn main() {
    // Get the encoded image path from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: decoder <path_to_encoded_image>");
        return;
    }
    let encoded_image_path = &args[1];

    // Generate the output file path with `decrypted_` prefix
    let output_data_path = create_output_path(encoded_image_path);

    // Load the encoded image
    let encoded_image = image::open(encoded_image_path)
        .expect("Failed to open encoded image")
        .to_rgba();

    // Create a decoder with the encoded image
    let decoder = decoder::Decoder::new(encoded_image);

    // Decode the hidden data
    let decoded_data = decoder.decode_alpha();  // or decoder.decode_image() for full RGBA decoding

    // Write the decoded data to the output file
    let mut output_file = File::create(&output_data_path).expect("Failed to create output file");
    output_file.write_all(&decoded_data).expect("Failed to write extracted data");

    println!("Hidden data extracted and saved to {}", output_data_path);
}

// Helper function to create output path with `decrypted_` prefix
fn create_output_path(encoded_image_path: &str) -> String {
    let path = Path::new(encoded_image_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let decrypted_file_name = format!("decrypted_{}", file_name);
    path.with_file_name(decrypted_file_name).to_string_lossy().into_owned()
}
