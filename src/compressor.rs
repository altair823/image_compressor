//! Module that contains things related with compressing a image.
//!
//! # Compressor
//!
//! The `compress_to_jpg` function resizes the given image and compresses it by a certain percentage.
//! # Examples
//! ```
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//! use image_compressor::Factor;
//!
//! let origin_dir = PathBuf::from("origin").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let compressor = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
//! compressor.compress_to_jpg();
//! ```

use crate::get_file_list;
use image::imageops::FilterType;
use mozjpeg::{ColorSpace, Compress, ScanMode};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

fn delete_duplicate_file<O: AsRef<Path>>(file_path: O) -> Result<O, Box<dyn Error>>
where
    std::path::PathBuf: PartialEq<O>,
{
    let current_dir_file_list = match get_file_list(file_path.as_ref().parent().unwrap()) {
        Ok(mut v) => {
            if let Some(index) = v.iter().position(|x| *x == file_path) {
                v.remove(index);
            }
            v
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let mut current_dir_file_list = current_dir_file_list
        .iter()
        .map(|p| p.file_stem().unwrap().to_str().unwrap());
    let t = file_path.as_ref().file_stem().unwrap().to_str().unwrap();
    if !current_dir_file_list.any(|x| x == t) {
        return Err(Box::new(io::Error::new(
            ErrorKind::NotFound,
            format!(
                "Cannot delete! The file {} can be the original file. ",
                file_path.as_ref().file_name().unwrap().to_str().unwrap()
            ),
        )));
    }

    match fs::remove_file(&file_path) {
        Ok(_) => (),
        Err(e) => return Err(Box::new(e)),
    }

    Ok(file_path)
}

/// Factor struct that used for setting quality and resize ratio in the new image.
///
/// The [`Compressor`] and [`FolderCompressor`](super::FolderCompressor) need a function pointer that
/// calculate and return the `Factor` for compressing images.
///
/// So, to create a new `Compressor` or `FolderCompressor` instance
/// you need to define a new function or closure that calculates and returns a `Factor` instance
/// based on the size of image(width and height) and file size.
///
/// The recommended range of quality is 60 to 80.
pub struct Factor {
    /// Quality of the new compressed image.
    /// Values range from 0 to 100 in float.
    quality: f32,

    /// Ratio for resize the new compressed image.
    /// Values range from 0 to 1 in float.
    size_ratio: f32,
}

impl Factor {
    /// Create a new `Factor` instance.
    /// The `quality` range from 0 to 100 in float,
    /// and `size_ratio` range from 0 to 1 in float.
    ///
    /// # Panics
    ///
    /// - If the quality value is 0 or less.
    /// - If the quality value exceeds 100.
    /// - If the size ratio value is 0 or less.
    /// - If the size ratio value exceeds 1.
    pub fn new(quality: f32, size_ratio: f32) -> Self {
        if (quality > 0. && quality <= 100.) && (size_ratio > 0. && size_ratio <= 1.) {
            Self {
                quality,
                size_ratio,
            }
        } else {
            panic!("Wrong Factor argument!");
        }
    }

    /// Getter for `quality` of `Factor`.
    pub fn quality(&self) -> f32 {
        self.quality
    }

    /// Getter for `size_ratio` of `Factor`.
    pub fn size_ratio(&self) -> f32 {
        self.size_ratio
    }
}

/// Compressor struct.
///
pub struct Compressor<O: AsRef<Path>, D: AsRef<Path>> {
    calculate_quality_and_size: fn(u32, u32, u64) -> Factor,
    original_path: O,
    destination_path: D,
    delete_orininal: bool,
}

impl<O: AsRef<Path>, D: AsRef<Path>> Compressor<O, D> {
    /// Create a new compressor.
    ///
    /// The new `Compressor` instance needs a function to calculate quality and scaling factor of the new compressed image.
    /// For more information of `cal_factor_func` parameter, please check the [`Factor`] struct.
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    /// use image_compressor::Factor;
    ///
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    ///
    /// let compressor = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
    /// ```
    pub fn new(origin_dir: O, dest_dir: D, cal_factor_func: fn(u32, u32, u64) -> Factor) -> Self {
        Compressor {
            calculate_quality_and_size: cal_factor_func,
            original_path: origin_dir,
            destination_path: dest_dir,
            delete_orininal: false,
        }
    }

    /// Sets whether the program deletes the original file.
    pub fn set_delete_origin(&mut self, to_delete: bool) {
        self.delete_orininal = to_delete;
    }

    fn convert_to_jpg(&self) -> Result<PathBuf, Box<dyn Error>> {
        let img = image::open(&self.original_path)?;
        let stem = self.original_path.as_ref().file_stem().unwrap();
        let mut new_path = match self.original_path.as_ref().parent() {
            Some(s) => s,
            None => {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Cannot get parent directory!",
                )))
            }
        }
        .join(stem);
        new_path.set_extension("jpg");
        img.save(&new_path)?;

        Ok(new_path)
    }

    fn compress(
        &self,
        resized_img_data: Vec<u8>,
        target_width: usize,
        target_height: usize,
        quality: f32,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut comp = Compress::new(ColorSpace::JCS_RGB);
        comp.set_scan_optimization_mode(ScanMode::Auto);
        comp.set_quality(quality);

        comp.set_size(target_width, target_height);

        comp.set_mem_dest();
        comp.set_optimize_scans(true);
        comp.start_compress();

        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(
                &resized_img_data[line * target_width * 3..(line + 1) * target_width * 3],
            );
            line += 1;
        }
        comp.finish_compress();

        let compressed = comp
            .data_to_vec()
            .map_err(|_| "data_to_vec failed".to_string())?;
        Ok(compressed)
    }

    fn resize(
        &self,
        path: &Path,
        resize_ratio: f32,
    ) -> Result<(Vec<u8>, usize, usize), Box<dyn Error>> {
        let img = image::open(path).map_err(|e| e.to_string())?;
        let width = img.width() as usize;
        let height = img.height() as usize;

        let width = width as f32 * resize_ratio;
        let height = height as f32 * resize_ratio;

        let resized_img = img.resize(width as u32, height as u32, FilterType::Triangle);

        let resized_width = resized_img.width() as usize;
        let resized_height = resized_img.height() as usize;

        Ok((
            resized_img.into_rgb8().into_vec(),
            resized_width,
            resized_height,
        ))
    }

    /// Compress a file.
    ///
    /// Compress the given image file and save it to target_dir.
    /// If the extension of the given image file is not jpg or jpeg, then convert the image to jpg file.
    /// If the module can not open the file, just copy it to target_dir.
    /// Compress quality and resize ratio calculate based on file size of the image.
    /// For a continuous multithreading process, every single error doesn't occur panic or exception and just print error message with return Ok.
    ///
    /// If the flag to delete the original is true, the function delete the original file.
    ///
    /// # Examples
    /// ```
    /// use std::path::PathBuf;
    /// use image_compressor::compressor::Compressor;
    /// use image_compressor::Factor;
    ///
    /// let origin_dir = PathBuf::from("origin").join("file1.jpg");
    /// let dest_dir = PathBuf::from("dest");
    ///
    /// let compressor = Compressor::new(origin_dir, dest_dir, |width, height, file_size| {return Factor::new(75., 0.7)});
    /// compressor.compress_to_jpg();
    /// ```
    pub fn compress_to_jpg(&self) -> Result<PathBuf, Box<dyn Error>> {
        let origin_file_path = self.original_path.as_ref();
        let target_dir = self.destination_path.as_ref();

        let file_name = match origin_file_path.file_name() {
            Some(e) => e.to_str().unwrap_or(""),
            None => "",
        };

        let file_stem = origin_file_path.file_stem().unwrap();
        let file_extension = match origin_file_path.extension() {
            None => OsStr::new(""),
            Some(e) => e,
        };

        let mut target_file_name = PathBuf::from(file_stem);
        target_file_name.set_extension("jpg");
        let target_file = target_dir.join(target_file_name);
        if target_dir.join(file_name).is_file() {
            return Err(Box::new(io::Error::new(
                ErrorKind::AlreadyExists,
                format!("The file is already existed! file: {}", file_name),
            )));
        }
        if target_file.is_file() {
            return Err(Box::new(io::Error::new(
                ErrorKind::AlreadyExists,
                format!(
                    "The compressed file is already existed! file: {}",
                    target_file.file_name().unwrap().to_str().unwrap()
                ),
            )));
        }

        let mut converted_file: Option<PathBuf> = None;

        let current_file;
        if file_extension.ne("jpg") && file_extension.ne("jpeg") {
            match self.convert_to_jpg() {
                Ok(p) => {
                    current_file = (&p).to_path_buf();
                    converted_file = Some(p);
                }
                Err(e) => {
                    let m = format!(
                        "Cannot convert file {} to jpg. Just copy it. : {}",
                        file_name, e
                    );
                    fs::copy(origin_file_path, target_dir.join(&file_name))?;
                    return Err(Box::new(io::Error::new(ErrorKind::InvalidData, m)));
                }
            };
        } else {
            current_file = self.original_path.as_ref().to_path_buf();
        }

        let image_file = image::open(&current_file)?;
        let width = image_file.width();
        let height = image_file.height();
        let file_size = match origin_file_path.metadata() {
            Ok(m) => m.len(),
            Err(_) => 0,
        };

        // Calculate factor.
        let factor = (self.calculate_quality_and_size)(width, height, file_size);

        let (resized_img_data, target_width, target_height) =
            self.resize(origin_file_path, factor.size_ratio())?;
        let compressed_img_data = self.compress(
            resized_img_data,
            target_width,
            target_height,
            factor.quality(),
        )?;

        let mut file = BufWriter::new(File::create(&target_file)?);
        file.write_all(&compressed_img_data)?;

        // Delete duplicate file which is converted from original one.
        match converted_file {
            Some(p) => {
                delete_duplicate_file(p)?;
            }
            None => {}
        }

        // Delete the original file when the flag is true.
        match self.delete_orininal {
            true => fs::remove_file(&self.original_path)?,
            false => (),
        }

        Ok(target_file)
    }
}
#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn setup(test_num: i32) -> (i32, PathBuf, PathBuf) {
        let test_origin_dir = PathBuf::from(&format!("{}{}", "test_origin", test_num));
        if test_origin_dir.is_dir() {
            fs::remove_dir_all(&test_origin_dir).unwrap();
        }
        fs::create_dir_all(&test_origin_dir.as_path()).unwrap();

        let test_dest_dir = PathBuf::from(&format!("{}{}", "test_dest", test_num));
        if test_dest_dir.is_dir() {
            fs::remove_dir_all(&test_dest_dir).unwrap();
        }
        fs::create_dir_all(&test_dest_dir.as_path()).unwrap();

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
    fn convert_to_jpg_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(1);

        fs::copy(
            "original_images/file1.png",
            test_origin_dir.join("file1.png"),
        )
        .unwrap();
        let compressor = Compressor::new(
            test_origin_dir.join("file1.png"),
            &test_dest_dir,
            |_, _, _| return Factor::new(70., 0.7),
        );
        assert_eq!(
            compressor.convert_to_jpg().unwrap(),
            test_origin_dir.join("file1.jpg")
        );

        fs::copy(
            "original_images/dir1/file5.webp",
            test_origin_dir.join("file5.webp"),
        )
        .unwrap();
        let compressor = Compressor::new(
            test_origin_dir.join("file5.webp"),
            &test_dest_dir,
            |_, _, _| return Factor::new(70., 0.7),
        );
        assert_eq!(
            compressor.convert_to_jpg().unwrap(),
            test_origin_dir.join("file5.jpg")
        );
        cleanup(1);
    }

    #[test]
    fn compress_a_image_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(2);
        let test_origin_path = test_origin_dir.join("file4.jpg");
        let test_dest_path = test_dest_dir.join("file4.jpg");

        fs::copy(Path::new("original_images/file4.jpg"), &test_origin_path).unwrap();

        let compressor = Compressor::new(
            test_origin_dir.join("file4.jpg"),
            test_dest_dir,
            |_, _, _| {
                return Factor::new(75., 0.7);
            },
        );

        compressor.compress_to_jpg().unwrap();

        assert!(test_dest_path.is_file());
        println!(
            "Original file size: {}, Compressed file size: {}",
            &test_origin_path.metadata().unwrap().len(),
            test_dest_path.metadata().unwrap().len()
        );
        cleanup(2);
    }

    #[test]
    fn compress_to_jpg_copy_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(3);
        fs::copy(
            "original_images/file7.txt",
            test_origin_dir.join("file7.txt"),
        )
        .unwrap();

        let compressor = Compressor::new(
            (&test_origin_dir).join("file7.txt"),
            &test_dest_dir,
            |_, _, _| {
                return Factor::new(75., 0.7);
            },
        );
        assert!(compressor.compress_to_jpg().is_err());
        assert!(test_dest_dir.join("file7.txt").is_file());
        cleanup(3);
    }

    #[test]
    fn delete_duplicate_file_test() {
        let (_, test_origin_dir, _) = setup(7);
        fs::copy(
            "original_images/file1.png",
            test_origin_dir.join("file1.png"),
        )
        .unwrap();
        fs::copy(
            "original_images/file2.jpg",
            test_origin_dir.join("file2.jpg"),
        )
        .unwrap();

        match delete_duplicate_file(test_origin_dir.join("file2.jpg")) {
            Ok(o) => println!("{}", o.to_str().unwrap()),
            Err(e) => println!("{}", e),
        }
        cleanup(7);
    }

    #[test]
    fn delete_original_test() {
        let (_, test_origin_dir, test_dest_dir) = setup(9);
        fs::copy(
            "original_images/file2.jpg",
            test_origin_dir.join("file2.jpg"),
        )
        .unwrap();

        let mut compressor = Compressor::new(
            test_origin_dir.join("file2.jpg"),
            test_dest_dir,
            |_, _, _| return Factor::new(75., 0.7),
        );
        compressor.set_delete_origin(true);
        compressor.compress_to_jpg().unwrap();
        cleanup(9);
    }
}
