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
//! base on image size and file size of the original image.
//! To see more information about it, see [`Factor`].
//!
//! # Examples
//!
//! ### `FolderCompressor` and its `compress` function example.
//!
//! The function compress all images in given origin folder with multithread at the same time,
//! and wait until everything is done.
//! If user set a [`Sender`] for [`FolderCompressor`], the method sends messages whether compressing is complete.
//! ```
//! use std::path::PathBuf;
//! use std::sync::mpsc;
//! use image_compressor::FolderCompressor;
//! use image_compressor::Factor;
//!
//! let origin = PathBuf::from("origin_dir");   // original directory path
//! let dest = PathBuf::from("dest_dir");       // destination directory path
//! let thread_count = 4;                       // number of threads
//! let (tx, tr) = mpsc::channel();             // Sender and Receiver. for more info, check mpsc and message passing.
//!
//! let mut comp = FolderCompressor::new(origin, dest);
//! comp.set_cal_func(|width, height, file_size| {return Factor::new(75., 0.7)});
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
//! let origin_dir = PathBuf::from("origin").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let comp = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
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
    original_path: PathBuf,
    destination_path: PathBuf,
    thread_count: u32,
    delete_original: bool,
    sender: Option<Sender<String>>,
}

impl FolderCompressor {
    /// Create a new `FolderCompressor` instance.
    /// Just needs original directory path and destination directory path.
    /// If you do not set the quality calculation function,
    /// it will use the default calculation function which sets the quality only by the file size.
    /// Likewise, if you do not set the number of threads, only one thread is used by default.\
    /// # Examples
    /// ```
    /// use image_compressor::FolderCompressor;
    /// use std::path::Path;
    ///
    /// let origin = Path::new("origin");
    /// let dest = Path::new("dest");
    ///
    /// let comp = FolderCompressor::new(origin, dest);
    /// ```
    pub fn new<O: AsRef<Path>, D: AsRef<Path>>(origin_path: O, dest_path: D) -> Self {
        FolderCompressor {
            factor: Factor::default(),
            original_path: origin_path.as_ref().to_path_buf(),
            destination_path: dest_path.as_ref().to_path_buf(),
            thread_count: 1,
            delete_original: false,
            sender: None,
        }
    }

    
    #[deprecated(since = "1.3.0", note = "Use just `set_factor` method instead this.")]
    pub fn set_cal_func(&mut self, cal_func: fn(u32, u32, u64) -> Factor) {
        self.factor = cal_func(0, 0, 0);
    }

    /// Set the factor for the quality and scale of the image.
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor;
    }

    /// Set whether to delete original files.
    pub fn set_delelte_origin(&mut self, to_delete: bool) {
        self.delete_original = to_delete;
    }

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
    /// let origin = Path::new("origin");
    /// let dest = Path::new("dest");
    ///
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_thread_count(4);
    /// ```
    pub fn set_thread_count(&mut self, thread_count: u32) {
        self.thread_count = thread_count;
    }

    /// Folder compress function.
    ///
    /// The function compress all images in given origin folder with multithreading, and wait until everything is done.
    /// If user set a [`Sender`] for [`FolderCompressor`] before, the method sends messages whether compressing is complete.
    ///
    /// # Warning
    /// Since this function comsume its `self`, the `FolderCompressor` instance (which is self) is no longer available after calling this function.
    /// ```
    /// use std::path::PathBuf;
    /// use std::sync::mpsc;
    /// use image_compressor::FolderCompressor;
    ///
    /// let origin = PathBuf::from("origin_dir");
    /// let dest = PathBuf::from("dest_dir");
    /// let (tx, tr) = mpsc::channel();
    ///
    /// let mut comp = FolderCompressor::new(origin, dest);
    /// comp.set_sender(tx);
    /// comp.set_thread_count(4);
    ///
    /// match comp.compress(){
    ///     Ok(_) => {},
    ///     Err(e) => println!("Cannot compress the folder!: {}", e),
    /// }
    /// ```
    pub fn compress(self) -> Result<(), Box<dyn Error>> {
        let to_comp_file_list = get_file_list(&self.original_path)?;
        try_send_message(
            &self.sender,
            format!("Total file count: {}", to_comp_file_list.len()),
        );

        let queue = Arc::new(SegQueue::new());
        for i in to_comp_file_list {
            queue.push(i);
        }
        let mut handles = Vec::new();
        let arc_root = Arc::new(self.original_path);
        let arc_dest = Arc::new(self.destination_path);
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
                            self.delete_original,
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
                        self.delete_original,
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

        if self.delete_original {
            match delete_recursive(&*arc_root) {
                Ok(_) => try_send_message(
                    &self.sender,
                    "Delete original directories complete!".to_string(),
                ),
                Err(e) => try_send_message(
                    &self.sender,
                    format!("Cannot delete original directories! {}", e),
                ),
            };
        }
        Ok(())
    }

    #[deprecated(since = "1.2.0", note = "Use just `compress` method instead this.")]
    pub fn compress_with_sender(self, sender: mpsc::Sender<String>) -> Result<(), Box<dyn Error>> {
        let to_comp_file_list = get_file_list(&self.original_path)?;
        send_message(
            &sender,
            format!("Total file count: {}", to_comp_file_list.len()),
        );

        let queue = Arc::new(SegQueue::new());
        for i in to_comp_file_list {
            queue.push(i);
        }
        let mut handles = Vec::new();
        let arc_root = Arc::new(self.original_path);
        let arc_dest = Arc::new(self.destination_path);
        for _ in 0..self.thread_count {
            let new_sender = sender.clone();
            let arc_root = Arc::clone(&arc_root);
            let arc_dest = Arc::clone(&arc_dest);
            let arc_queue = Arc::clone(&queue);
            let arc_factor = Arc::new(self.factor);
            let handle = thread::spawn(move || {
                process_with_sender(
                    arc_queue,
                    &arc_root,
                    &arc_dest,
                    self.delete_original,
                    *arc_factor.clone(),
                    new_sender,
                );
            });
            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        send_message(&sender, "Compress complete!".to_string());

        if self.delete_original {
            match delete_recursive(&*arc_root) {
                Ok(_) => send_message(&sender, "Delete original directories complete!".to_string()),
                Err(e) => send_message(
                    &sender,
                    format!("Cannot delete original directories! {}", e),
                ),
            };
        }
        Ok(())
    }
}

fn process(
    queue: Arc<SegQueue<PathBuf>>,
    root: &Path,
    dest: &Path,
    to_delete_original: bool,
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
                compressor.set_delete_origin(to_delete_original);
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

fn process_with_sender(
    queue: Arc<SegQueue<PathBuf>>,
    root: &Path,
    dest: &Path,
    to_delete_original: bool,
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
                compressor.set_delete_origin(to_delete_original);
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
    use fs_extra::dir;
    use fs_extra::dir::CopyOptions;
    use std::fs;

    fn setup(test_num: i32) -> (i32, PathBuf, PathBuf) {
        let test_origin_dir = PathBuf::from(&format!("{}{}", "test_origin", test_num));
        if test_origin_dir.is_dir() {
            fs::remove_dir_all(&test_origin_dir).unwrap();
        }
        fs::create_dir(&test_origin_dir.as_path()).unwrap();

        let test_dest_dir = PathBuf::from(&format!("{}{}", "test_dest", test_num));
        if test_dest_dir.is_dir() {
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
        fs::create_dir(&test_dest_dir.as_path()).unwrap();

        let options = CopyOptions::new();
        dir::copy("original_images", &test_origin_dir, &options).unwrap();

        (test_num, test_origin_dir, test_dest_dir)
    }

    fn cleanup(test_num: i32) {
        let test_origin_dir = PathBuf::from(&format!("{}{}", "test_origin", test_num));
        if test_origin_dir.is_dir() {
            fs::remove_dir_all(&test_origin_dir).unwrap();
        }
        let test_dest_dir = PathBuf::from(&format!("{}{}", "test_dest", test_num));
        if test_dest_dir.is_dir() {
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
    }

    #[test]
    fn folder_compress_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(4);
        let mut folder_compressor = FolderCompressor::new(&test_origin_dir, &test_dest_dir);
        folder_compressor.set_thread_count(4);
        folder_compressor.compress().unwrap();
        let a = get_file_list(test_origin_dir).unwrap();
        let b = get_file_list(test_dest_dir).unwrap();
        let origin_file_list = a.iter().map(|i| i.file_stem().unwrap()).collect::<Vec<_>>();
        let dest_file_list = b.iter().map(|i| i.file_stem().unwrap()).collect::<Vec<_>>();
        assert_eq!(origin_file_list, dest_file_list);
        cleanup(4);
    }

}
