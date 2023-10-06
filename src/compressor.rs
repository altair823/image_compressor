//! Module that contains things related with compressing a image.
//!
//! # Compressor
//!
//! The `compress_to_jpg` function resizes the given image and compresses it by a certain percentage.
//! # Examples
//! ```rust,no_run
//! use std::path::PathBuf;
//! use image_compressor::compressor::Compressor;
//! use image_compressor::Factor;
//!
//! let source_file = PathBuf::from("source").join("file1.jpg");
//! let dest_dir = PathBuf::from("dest");
//!
//! let mut compressor = Compressor::new(source_file, dest_dir);
//! compressor.set_factor(Factor::new(80., 0.8));
//! compressor.compress_to_jpg();
//! ```

use image::imageops::FilterType;
use mozjpeg::{ColorSpace, Compress, ScanMode};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Factor struct that used for setting quality and resize ratio in the new image.
///
/// The [`Compressor`] and [`FolderCompressor`](super::FolderCompressor) need `Factor` for compressing images.
///
/// So, to create a new `Compressor` or `FolderCompressor` instance
/// you need to define a new `Factor` instance contains the quality ratio of image and file size ratio to compress.
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
    source_path: O,
    dest_path: D,
    delete_source: bool,
    overwrite_dest: bool,
}

impl<O: AsRef<Path>, D: AsRef<Path>> Compressor<O, D> {
    /// Create a new `Compressor` instance.
    pub fn new(source_dir: O, dest_dir: D) -> Self {
        Compressor {
            factor: Factor::default(),
            source_path: source_dir,
            dest_path: dest_dir,
            delete_source: false,
            overwrite_dest: false,
        }
    }

    /// Set factor for the new compressed image.
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor;
    }

    /// Sets whether the program deletes the source file.
    pub fn set_delete_source(&mut self, to_delete: bool) {
        self.delete_source = to_delete;
    }

    /// Sets whether the program overwrites the destination file.
    pub fn set_overwrite_dest(&mut self, to_overwrite: bool) {
        self.overwrite_dest = to_overwrite;
    }

    /// Compress the image to jpg format.
    /// The new image will be saved in the destination directory.
    ///
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

        comp.set_optimize_scans(true);
        let mut comp = comp.start_compress(Vec::new())?;

        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(
                &resized_img_data[line * target_width * 3..(line + 1) * target_width * 3],
            )?;
            line += 1;
        }

        let compressed = comp.finish()?;
        Ok(compressed)
    }

    /// Convert RGBA8 to RGB8.
    /// If the alpha channel is 0, the function convert the pixel to white.
    /// Otherwise, the function just remove the alpha channel.
    fn rgba8_to_rgb8(rgba_data: &[u8]) -> Vec<u8> {
        let mut rgb_data = Vec::with_capacity((rgba_data.len() / 4) * 3);
        for i in (0..rgba_data.len()).step_by(4) {
            if rgba_data[i + 3] == 0 {
                rgb_data.push(255); // R
                rgb_data.push(255); // G
                rgb_data.push(255); // B
            } else {
                rgb_data.push(rgba_data[i]); // R
                rgb_data.push(rgba_data[i + 1]); // G
                rgb_data.push(rgba_data[i + 2]); // B
            }
        }
        rgb_data
    }

    /// Resize the image vector.
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

        // Map to RGB8
        let resized_img = match resized_img.color() {
            image::ColorType::Rgb8 => resized_img.into_rgb8().to_vec(),
            _ => Self::rgba8_to_rgb8(resized_img.to_rgba8().as_raw()),
        };

        Ok((resized_img, resized_width, resized_height))
    }

    /// Compress a file.
    ///
    /// Compress the given image file and save it to target_dir.
    /// If the extension of the given image file is not jpg or jpeg, convert the image to jpg file.
    /// If the image module can not open the file, such as pdf, mp4, etc., just copy it to target_dir.
    /// Compress quality and resize ratio calculate based on file size of the image.
    /// For a continuous multithreading process, every single error doesn't occur panic or exception and just print error message with return Ok.
    ///
    /// If the flag to delete the source is true, the function delete the source file.
    ///
    pub fn compress_to_jpg(&self) -> Result<PathBuf, Box<dyn Error>> {
        let source_file_path = self.source_path.as_ref();
        let target_dir = self.dest_path.as_ref();

        let file_stem = source_file_path.file_stem().unwrap();

        let mut target_file_name = PathBuf::from(file_stem);
        target_file_name.set_extension("jpg");
        let target_file = target_dir.join(&target_file_name);

        // If the target file is already existed and the flag to overwrite is false, return error.
        if target_file.is_file() && !self.overwrite_dest {
            return Err(Box::new(io::Error::new(
                ErrorKind::AlreadyExists,
                format!(
                    "The compressed file is already existed! file: {}",
                    target_file.file_name().unwrap().to_str().unwrap()
                ),
            )));
        }

        let (resized_img_data, target_width, target_height) =
            self.resize(source_file_path, self.factor.size_ratio())?;
        let compressed_img_data = self.compress(
            resized_img_data,
            target_width,
            target_height,
            self.factor.quality(),
        )?;

        let mut file = BufWriter::new(File::create(&target_file)?);
        file.write_all(&compressed_img_data)?;

        // Delete the source file when the flag is true.
        match self.delete_source {
            true => {
                if self.source_path.as_ref() != target_file {
                    fs::remove_file(&self.source_path)?
                }
            }
            false => (),
        }

        Ok(target_file)
    }
}
#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    use colorgrad;
    use image::ImageBuffer;
    use rand::Rng;
    use std::path::{Path, PathBuf};

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
    fn skip_wrong_ext_test() {
        let (test_dir, _) = setup("skip_wrong_ext_test_dir");
        let txt_data = "Hello, World!";
        let mut txt_path = PathBuf::from(&test_dir).join("skip_wrong_ext_test.txt");
        let mut txt_file = File::create(&txt_path).unwrap();
        write!(txt_file, "{}", txt_data).unwrap();

        let compressor = Compressor::new(&txt_path, &test_dir);
        assert!(compressor.compress_to_jpg().is_err());
        assert!(txt_path.is_file());
        txt_path.set_extension("jpg");
        assert!(!txt_path.is_file());
        cleanup(test_dir);
    }

    #[test]
    fn compress_to_jpg_test() {
        let (test_dir, mut test_images) = setup("compress_to_jpg_test");

        let dest_dir = PathBuf::from("compress_to_jpg_dest_dir");
        fs::create_dir_all(&dest_dir).unwrap();

        for test_image in &test_images {
            let mut compressor = Compressor::new(test_image, &dest_dir);
            compressor.set_factor(Factor::new(0.5, 0.5));
            compressor.compress_to_jpg().unwrap();
        }
        test_images = test_images
            .iter()
            .map(|image| dest_dir.join(image.file_name().unwrap()))
            .collect();
        for new_image in &test_images {
            let mut new_test_image = new_image.clone();
            new_test_image.set_extension("jpg");
            assert!(new_test_image.is_file());
        }
        cleanup(test_dir);
        cleanup(dest_dir);
    }

    #[test]
    fn compress_to_jpg_with_delete_test() {
        let (test_dir, mut test_images) = setup("compress_to_jpg_with_delete_test");

        let dest_dir = PathBuf::from("compress_to_jpg_with_delete_dest_dir");
        fs::create_dir_all(&dest_dir).unwrap();

        for test_image in &test_images {
            let mut compressor = Compressor::new(test_image, &dest_dir);
            compressor.set_delete_source(true);
            compressor.compress_to_jpg().unwrap();
        }
        for test_image in &test_images {
            assert!(!test_image.is_file());
        }
        test_images = test_images
            .iter()
            .map(|image| dest_dir.join(image.file_name().unwrap()))
            .collect();
        for new_image in &test_images {
            let mut new_test_image = new_image.clone();
            new_test_image.set_extension("jpg");
            assert!(new_test_image.is_file());
        }
        cleanup(test_dir);
        cleanup(dest_dir);
    }
}
