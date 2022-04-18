# Image Compressor

[![Crates.io](https://img.shields.io/crates/v/image_compressor.svg)](https://crates.io/crates/image_compressor)  [![Documentation](https://docs.rs/image/badge.svg)](https://docs.rs/image_compressor/)

A library for resize and compress images to jpg.

## Features

- Compress and resize a single image to jpg format. 
- Compress and resize multiple images of certain directory. 
- Compress and resize images with multithreading. 
- Customize the quality and size of compressed images. 
- Send a completion message via `mpsc::Sender` (see [Using Message Passing to Transfer Data Between Threads](https://doc.rust-lang.org/book/ch16-02-message-passing.html) in rust tutorial).

## Supported Image Format

Visit [image](https://crates.io/crates/image) crate page. 
This crate uses image crate for opening image files. 

## Examples

#### `FolderCompressor` and its `compress` function example.

The function compress all images in given origin folder with multithread at the same time,
and wait until everything is done. 
If user set a `Sender` for `FolderCompressor`, the method sends messages whether compressing is complete. 

```rust
use std::path::PathBuf;
use std::sync::mpsc;
use image_compressor::FolderCompressor;
use image_compressor::Factor;

let origin = PathBuf::from("origin_dir");   // original directory path
let dest = PathBuf::from("dest_dir");       // destination directory path
let thread_count = 4;                       // number of threads
let (tx, tr) = mpsc::channel();             // Sender and Receiver. for more info, check mpsc and message passing. 

let mut comp = FolderCompressor::new(origin, dest);
comp.set_cal_func(|width, height, file_size| {return Factor::new(75., 0.7)});
comp.set_thread_count(4);
comp.set_sender(tx);

match comp.compress(){
    Ok(_) => {},
    Err(e) => println!("Cannot compress the folder!: {}", e),
}
```

#### `Compressor` and `compress_to_jpg` example.

Compressing just a one image. 
```rust
use std::path::PathBuf;
use image_compressor::compressor::Compressor;
use image_compressor::Factor;
let origin = PathBuf::from("origin").join("file1.jpg");
let dest = PathBuf::from("dest");
let comp = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
comp.compress_to_jpg();
```