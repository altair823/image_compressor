# Image Compressor

[![Crates.io](https://img.shields.io/crates/v/image_compressor.svg)](https://crates.io/crates/image_compressor)  [![Documentation](https://docs.rs/image/badge.svg)](https://docs.rs/image_compressor/)

A library for resizing and compressing images to **jpg**.

## Features

- Compress and resize a single image to jpg format. 
- Multithreading. 
- Customize the quality and size ratio of compressed images. 
- Send a completion message via `mpsc::Sender` (see [Using Message Passing to Transfer Data Between Threads](https://doc.rust-lang.org/book/ch16-02-message-passing.html) in rust tutorial).

## Supported Image Format

Visit [image](https://crates.io/crates/image) crate page. 
This crate uses image crate for opening image files. 

## Examples

#### `FolderCompressor` and its `compress` function example.

The function compress all images in given source folder with multithread at the same time,
and wait until everything is done. 
If user set a `Sender` for `FolderCompressor`, the method sends messages whether compressing is complete. 

```rust
use std::path::PathBuf;
use std::sync::mpsc;
use image_compressor::FolderCompressor;
use image_compressor::Factor;

let source = PathBuf::from("source_dir");   // source directory path
let dest = PathBuf::from("dest_dir");       // destination directory path
let thread_count = 4;                       // number of threads
let (tx, tr) = mpsc::channel();             // Sender and Receiver. for more info, check mpsc and message passing. 

let mut comp = FolderCompressor::new(source, dest);
comp.set_factor(Factor::new(80., 0.8));
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

let source = PathBuf::from("source").join("file1.jpg");
let dest = PathBuf::from("dest");
let comp = Compressor::new(source_dir, dest_dir);
compressor.set_factor(Factor::new(80., 0.8));
comp.compress_to_jpg();
```