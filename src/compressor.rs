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


/// Why does this function exist?
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
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
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

impl Default for Factor {
    fn default() -> Self {
        Self {
            quality: 80.,
            size_ratio: 0.8,
        }
    }
}

/// Compressor struct.
///
pub struct Compressor<O: AsRef<Path>, D: AsRef<Path>> {
    factor: Factor,
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
    pub fn new(origin_dir: O, dest_dir: D) -> Self {
        Compressor {
            factor: Factor::default(),
            original_path: origin_dir,
            destination_path: dest_dir,
            delete_orininal: false,
        }
    }

    /// Set factor for the new compressed image.
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor;
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
        let target_file = target_dir.join(&target_file_name);
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

        if file_extension.ne("jpg") && file_extension.ne("jpeg") {
            converted_file = match self.convert_to_jpg() {
                Ok(p) => {
                    Some(p)
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
        } 

        let (resized_img_data, target_width, target_height) =
            self.resize(origin_file_path, self.factor.size_ratio())?;
        let compressed_img_data = self.compress(
            resized_img_data,
            target_width,
            target_height,
            self.factor.quality(),
        )?;

        let mut file = BufWriter::new(File::create(&target_file)?);
        file.write_all(&compressed_img_data)?;

        match converted_file {
            Some(c) => {
                fs::remove_file(c)?; 
            },
            None => (),
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

    use image::{ImageBuffer, io::Reader, ImageFormat};
    use colorgrad;
    use rand::Rng;
    use std::path::{Path, PathBuf};


    /// Create test directory and a image file in it. 
    fn setup(test_name: &str) -> (PathBuf, Vec<PathBuf>) {
        let test_dir = PathBuf::from(test_name);
        if test_dir.is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
        fs::create_dir_all(&test_dir.as_path()).unwrap();

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
        .build().unwrap();
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
    fn convert_to_jpg_test() {
        const COMPRESSOR_TEST_DIR: &str = "convert_to_jpg_test_dir";
        
        let (test_dir, test_images) = setup(COMPRESSOR_TEST_DIR);

        for test_image in &test_images {
            let compressor = Compressor::new(
                test_image,
                &test_dir,
            );
            let result = compressor.convert_to_jpg().unwrap(); // Just convert, not compress. 
            assert_eq!(Reader::open(result).unwrap().with_guessed_format().unwrap().format().unwrap(), ImageFormat::Jpeg);
        }
        cleanup(test_dir);
    }

    #[test]
    fn skip_wrong_ext_test() {
        const COMPRESSOR_TEST_DIR: &str = "skip_wrong_ext_test_dir";
        let (test_dir, _) = setup(COMPRESSOR_TEST_DIR);
        let txt_data = "Hello, World!";
        let mut txt_path = PathBuf::from(COMPRESSOR_TEST_DIR).join("skip_wrong_ext_test.txt");
        let mut txt_file = File::create(&txt_path).unwrap();
        write!(txt_file, "{}", txt_data).unwrap();

        let compressor = Compressor::new(
            &txt_path,
            &test_dir,
        );
        assert!(compressor.compress_to_jpg().is_err());
        assert!(txt_path.is_file());
        txt_path.set_extension("jpg");
        assert!(!txt_path.is_file());
        cleanup(test_dir);
    }

    #[test]
    fn compress_to_jpg_test() {
        const COMPRESSOR_TEST_DIR: &str = "compress_to_jpg_test";
        let (test_dir, mut test_images) = setup(COMPRESSOR_TEST_DIR);

        let dest_dir = PathBuf::from("compress_to_jpg_dest_dir");
        fs::create_dir_all(&dest_dir).unwrap();

        for test_image in &test_images {
            let mut compressor = Compressor::new(
                test_image,
                &dest_dir,
            );
            compressor.set_delete_origin(true);
            compressor.compress_to_jpg().unwrap();
        }
        for test_image in &test_images {
            assert!(!test_image.is_file());
        }
        test_images = test_images.iter().map(|image| {
            dest_dir.join(image.file_name().unwrap())
        }).collect();
        for new_image in &test_images {
            let mut new_test_image = new_image.clone();
            new_test_image.set_extension("jpg");
            assert!(new_test_image.is_file());
        }
        cleanup(test_dir);
        cleanup(dest_dir);
    }
}
