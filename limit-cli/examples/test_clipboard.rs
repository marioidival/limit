//! Test clipboard functionality
//! Run with: cargo run --example test_clipboard -p limit-cli

use arboard::Clipboard;
use image::RgbaImage;

fn main() {
    println!("=== Clipboard Test ===\n");

    let mut cb = match Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            eprintln!("✗ Failed to open clipboard: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Clipboard opened successfully\n");

    // Test 1: Image data
    println!("1. Testing get_image()...");
    match cb.get_image() {
        Ok(img) => {
            println!("   ✓ Image data found!");
            println!("     - Dimensions: {}x{}", img.width, img.height);
            println!(
                "     - Bytes: {} (expected: {})",
                img.bytes.len(),
                img.width * img.height * 4
            );

            // Verify if it's a valid RGBA buffer
            if let Some(_rgba) =
                RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
            {
                println!("   ✓ Valid RGBA image");
            } else {
                println!("   ✗ Invalid RGBA buffer");
            }
        }
        Err(e) => {
            println!("   ✗ No image data: {}", e);
        }
    }

    // Test 2: File list
    println!("\n2. Testing file_list()...");
    match cb.get().file_list() {
        Ok(files) => {
            if files.is_empty() {
                println!("   ✓ File list is empty");
            } else {
                println!("   ✓ Found {} file(s):", files.len());
                for f in &files {
                    println!("     - {:?}", f);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
        }
    }

    // Test 3: Try to read as text
    println!("\n3. Testing get_text()...");
    match cb.get_text() {
        Ok(text) => {
            if text.is_empty() {
                println!("   ✓ Text is empty");
            } else {
                println!("   ✓ Text: {} chars", text.len());
                if text.len() < 100 {
                    println!("     \"{}\"", text);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
        }
    }

    println!("\n=== Test Complete ===");
    println!("\nCopie uma imagem para o clipboard e execute novamente!");
}
