//! # Image compressor
//!
//! `image_compressor` is a library that compresses images with multiple threads.
//! See [image](https://crates.io/crates/image) crate for check the extention that supported.
//!
//! If you want to compress a single image, see [`Compressor`](compressor::Compressor) struct.
//!
//! Or if you want to compress multiple images in a certain directory, see [`FolderCompressor`] struct.
//! It compresses images by using multithread.
//!
//! To use these structs and its functions, you need to give them a function pointer or closure
//! that calculate size and quality of new compressed images.
//! That calculator function(or closure) need to calculate and returns a [`Factor`]
//! base on image size and file size of the source image.
//! To see more information about it, see [`Factor`].
//!
//! # Examples
//!
//! ### `FolderCompressor` and its `compress` function example.
//!
//! The function compress all images in given source folder with multithread at the same time,
//! and wait until everything is done.
//! If user set a [`Sender`] for [`FolderCompressor`], the method sends messages whether compressing is complete.
//! ```
//! use std::path::PathBuf;
//! use std::sync::mpsc;
//! use image_compressor::FolderCompressor;
//! use image_compressor::Factor;
//!
//! let source = PathBuf::from("source_dir");   // source directory path
//! let dest = PathBuf::from("dest_dir");       // destination directory path
//! let thread_count = 4;                       // number of threads
//! let (tx, tr) = mpsc::channel();             // Sender and Receiver. for more info, check mpsc and message passing.
//!
//! let mut comp = FolderCompressor::new(source, dest);
//! comp.set_factor(Factor::new(80., 0.8));
//! comp.set_thread_count(4);
//! comp.set_sender(tx);
//!
//! match comp.compress(){
//!     Ok(_) => {},
//!     Err(e) => println!("Cannot compress the folder!: {}", e),
//! }
//! ```
//!
//! ### `Compressor` and `compress_to_jpg` example.
//!
//! Compressing just a one image.
//! ```
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//! use image_compressor::Factor;
//!
//! let source_dir = PathBuf::from("source").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let mut comp = Compressor::new(source_dir, dest_dir);
//! comp.set_factor(Factor::new(80., 0.8));
//! comp.compress_to_jpg();
//! ```

use compressor::Compressor;
use crawler::get_file_list;
use crossbeam_queue::SegQueue;
use dir::delete_recursive;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc};
use std::thread;

pub mod compressor;
pub mod crawler;
pub mod dir;

pub use compressor::Factor;

fn try_send_message<T: ToString>(sender: &Option<Sender<T>>, message: T) {
    match sender {
        Some(s) => send_message(s, message),
        None => (),
    }
}

fn send_message<T: ToString>(sender: &Sender<T>, message: T) {
    match sender.send(message) {
        Ok(_) => (),
        Err(e) => println!("Message passing error!: {}", e),
    }
}

/// Compressor struct for a directory.
pub struct FolderCompressor {
    factor: Factor,
    source_path: PathBuf,
    dest_path: PathBuf,
    thread_count: u32,
    delete_source: bool,
    sender: Option<Sender<String>>,
}

impl FolderCompressor {
    /// Create a new `FolderCompressor` instance.
    /// Just needs source directory path and destination directory path.
    /// If you do not set the quality calculation function,
    /// it will use the default calculation function which sets the quality only by the file size.
    /// Likewise, if you do not set the number of threads, only one thread is used by default.\
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use std::path::Path;
    ///
    /// let source = Path::new("source");
    /// let dest = Path::new("dest");
    ///
    /// let comp = FolderCompressor::new(source, dest);
    /// ```
    pub fn new<O: AsRef<Path>, D: AsRef<Path>>(source_path: O, dest_path: D) -> Self {
        FolderCompressor {
            factor: Factor::default(),
            source_path: source_path.as_ref().to_path_buf(),
            dest_path: dest_path.as_ref().to_path_buf(),
            thread_count: 1,
            delete_source: false,
            sender: None,
        }
    }

    /// Set Factor using to compress images.
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor;
    }

    /// Set whether to delete source files.
    pub fn set_delete_source(&mut self, to_delete: bool) {
        self.delete_source = to_delete;
    }

    /// Set Sender for message passing.
    /// If you set a sender, the method sends messages whether compressing is complete.
    pub fn set_sender(&mut self, sender: Sender<String>) {
        self.sender = Some(sender);
    }

    /// Setter for the number of threads used to compress images.
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use image_compressor::Factor;
    /// use std::path::Path;
    ///
    /// let source = Path::new("source");
    /// let dest = Path::new("dest");
    ///
    /// let mut comp = FolderCompressor::new(source, dest);
    /// comp.set_thread_count(4);
    /// ```
    pub fn set_thread_count(&mut self, thread_count: u32) {
        self.thread_count = thread_count;
    }

    /// Folder compress function.
    ///
    /// The function compress all images in given source folder with multithreading, and wait until everything is done.
    /// If user set a [`Sender`] for [`FolderCompressor`] before, the method sends messages whether compressing is complete.
    ///
    /// # Warning
    /// Since this function comsume its `self`, the `FolderCompressor` instance (which is self) is no longer available after calling this function.
    /// ```
    /// use std::path::PathBuf;
    /// use std::sync::mpsc;
    /// use image_compressor::FolderCompressor;
    ///
    /// let source = PathBuf::from("source_dir");
    /// let dest = PathBuf::from("dest_dir");
    /// let (tx, tr) = mpsc::channel();
    ///
    /// let mut comp = FolderCompressor::new(source, dest);
    /// comp.set_sender(tx);
    /// comp.set_thread_count(4);
    ///
    /// match comp.compress(){
    ///     Ok(_) => {},
    ///     Err(e) => println!("Cannot compress the folder!: {}", e),
    /// }
    /// ```
    pub fn compress(self) -> Result<(), Box<dyn Error>> {
        let to_comp_file_list = get_file_list(&self.source_path)?;
        try_send_message(
            &self.sender,
            format!("Total file count: {}", to_comp_file_list.len()),
        );

        let queue = Arc::new(SegQueue::new());
        for i in to_comp_file_list {
            queue.push(i);
        }
        let mut handles = Vec::new();
        let arc_root = Arc::new(self.source_path);
        let arc_dest = Arc::new(self.dest_path);
        for _ in 0..self.thread_count {
            let arc_root = Arc::clone(&arc_root);
            let arc_dest = Arc::clone(&arc_dest);
            let arc_queue = Arc::clone(&queue);
            let arc_factor = Arc::new(self.factor);
            let handle = match self.sender {
                Some(ref s) => {
                    let new_s = s.clone();
                    thread::spawn(move || {
                        process_with_sender(
                            arc_queue,
                            &arc_root,
                            &arc_dest,
                            self.delete_source,
                            *arc_factor.clone(),
                            new_s,
                        );
                    })
                }
                None => thread::spawn(move || {
                    process(
                        arc_queue,
                        &arc_root,
                        &arc_dest,
                        self.delete_source,
                        *arc_factor.clone(),
                    );
                }),
            };
            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        try_send_message(&self.sender, "Compress complete!".to_string());

        if self.delete_source {
            match delete_recursive(&*arc_root) {
                Ok(_) => try_send_message(
                    &self.sender,
                    "Delete source directories complete!".to_string(),
                ),
                Err(e) => try_send_message(
                    &self.sender,
                    format!("Cannot delete source directories! {}", e),
                ),
            };
        }
        Ok(())
    }
}

/// Process function for multithread compressing.
/// This function is used when user doesn't set a [`Sender`] for [`FolderCompressor`].
fn process(
    queue: Arc<SegQueue<PathBuf>>,
    root: &Path,
    dest: &Path,
    to_delete_source: bool,
    factor: Factor,
) {
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_name = match file.file_name() {
                    None => "",
                    Some(s) => match s.to_str() {
                        None => "",
                        Some(s) => s,
                    },
                };
                let parent = match file.parent() {
                    Some(p) => match p.strip_prefix(root) {
                        Ok(p) => p,
                        Err(_) => {
                            println!("Cannot strip the prefix of file {}", file_name);
                            continue;
                        }
                    },
                    None => {
                        println!("Cannot find the parent directory of file {}", file_name);
                        continue;
                    }
                };
                let new_dest_dir = dest.join(parent);
                if !new_dest_dir.is_dir() {
                    match fs::create_dir_all(&new_dest_dir) {
                        Ok(_) => {}
                        Err(_) => {
                            println!("Cannot create the parent directory of file {}", file_name);
                            continue;
                        }
                    };
                }
                let mut compressor = Compressor::new(&file, new_dest_dir);
                compressor.set_factor(factor);
                compressor.set_delete_source(to_delete_source);
                match compressor.compress_to_jpg() {
                    Ok(_) => {
                        println!("Compress complete! File: {}", file_name);
                    }
                    Err(e) => {
                        println!("Cannot compress image file {} : {}", file_name, e);
                    }
                };
            }
        }
    }
}

/// Process function for multithread compressing.
/// This function is used when user sets a [`Sender`] for [`FolderCompressor`].
/// This function sends messages to the [`Sender`] when compressing is complete.
fn process_with_sender(
    queue: Arc<SegQueue<PathBuf>>,
    root: &Path,
    dest: &Path,
    to_delete_source: bool,
    factor: Factor,
    sender: mpsc::Sender<String>,
) {
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_name = match file.file_name() {
                    None => "",
                    Some(s) => match s.to_str() {
                        None => "",
                        Some(s) => s,
                    },
                };
                let parent = match file.parent() {
                    Some(p) => match p.strip_prefix(root) {
                        Ok(p) => p,
                        Err(_) => {
                            println!("Cannot strip the prefix of file {}", file_name);
                            continue;
                        }
                    },
                    None => {
                        println!("Cannot find the parent directory of file {}", file_name);
                        continue;
                    }
                };
                let new_dest_dir = dest.join(parent);
                if !new_dest_dir.is_dir() {
                    match fs::create_dir_all(&new_dest_dir) {
                        Ok(_) => {}
                        Err(_) => {
                            println!("Cannot create the parent directory of file {}", file_name);
                            continue;
                        }
                    };
                }
                let mut compressor = Compressor::new(&file, new_dest_dir);
                compressor.set_factor(factor);
                compressor.set_delete_source(to_delete_source);
                match compressor.compress_to_jpg() {
                    Ok(p) => send_message(
                        &sender,
                        format!(
                            "Compress complete! File: {}",
                            p.file_name().unwrap().to_str().unwrap()
                        ),
                    ),
                    Err(e) => send_message(&sender, e.to_string()),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;
    use rand::Rng;
    use std::fs;

    /// Create test directory and a image file in it.
    fn setup<T: AsRef<Path>>(test_name: T) -> (PathBuf, Vec<PathBuf>) {
        let test_dir = test_name.as_ref().to_path_buf();
        if test_dir.is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
        fs::create_dir_all(&test_dir).unwrap();

        const WIDTH: u32 = 256;
        const HEIGHT: u32 = 256;
        let img_stripe = ImageBuffer::from_fn(WIDTH, HEIGHT, |x, _| {
            if x % 2 == 0 {
                image::Luma([0u8])
            } else {
                image::Luma([255u8])
            }
        });
        let stripe_path = test_dir.join("img_stripe.png");
        img_stripe.save(&stripe_path).unwrap();
        let img_random_rgb = ImageBuffer::from_fn(WIDTH, HEIGHT, |_, _| {
            let r = rand::thread_rng().gen_range(0..256) as u8;
            let g = rand::thread_rng().gen_range(0..256) as u8;
            let b = rand::thread_rng().gen_range(0..256) as u8;
            image::Rgb([r, g, b])
        });
        let rgb_path = test_dir.join("img_random_rgb.gif");
        img_random_rgb.save(&rgb_path).unwrap();
        let grad = colorgrad::CustomGradient::new()
            .html_colors(&["deeppink", "gold", "seagreen"])
            .build()
            .unwrap();
        let mut img_jpg = ImageBuffer::new(WIDTH, HEIGHT);
        for (x, _, pixel) in img_jpg.enumerate_pixels_mut() {
            let rgba = grad.at(x as f64 / WIDTH as f64).to_rgba8();
            *pixel = image::Rgba(rgba);
        }
        let jpg_path = test_dir.join("img_jpg.jpg");
        img_jpg.save(&jpg_path).unwrap();
        (test_dir, vec![stripe_path, rgb_path, jpg_path])
    }

    fn cleanup<T: AsRef<Path>>(test_dir: T) {
        if test_dir.as_ref().is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
    }

    #[test]
    fn folder_compress_test() {
        let (test_source_dir, _) = setup("folder_compress_test_source");
        let test_dest_dir = PathBuf::from("folder_compress_test_dest");
        if test_dest_dir.is_dir() {
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
        fs::create_dir_all(&test_dest_dir).unwrap();

        let mut folder_compressor = FolderCompressor::new(&test_source_dir, &test_dest_dir);
        folder_compressor.set_thread_count(4);
        folder_compressor.compress().unwrap();
        let a = get_file_list(&test_source_dir).unwrap();
        let b = get_file_list(&test_dest_dir).unwrap();
        let mut source_file_list = a.iter().map(|i| i.file_stem().unwrap()).collect::<Vec<_>>();
        let mut dest_file_list = b.iter().map(|i| i.file_stem().unwrap()).collect::<Vec<_>>();
        source_file_list.sort();
        dest_file_list.sort();
        assert_eq!(source_file_list, dest_file_list);
        cleanup(test_source_dir);
        cleanup(test_dest_dir);
    }
}
