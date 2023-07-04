//! Functions related to directory manipulation.
//!

use std::error::Error;
use std::fs::{read_dir, remove_dir_all};
use std::io;
use std::path::Path;

/// Delete directories that are empty.
///
/// If the directory is not empty, this function doesn't delete that particular directory and its parents also.
///
/// # Error
/// - When directory is not empty.
/// - When the child directory is not empty.
///
/// # Examples
/// ```
/// use image_compressor::dir::delete_recursive;
/// use std::fs;
/// use std::path::Path;
/// use fs_extra::dir;
/// use fs_extra::dir::CopyOptions;
///
/// let test_origin_dir = Path::new("test_origin");
/// if test_origin_dir.is_dir(){
///     fs::remove_dir_all(test_origin_dir).unwrap();
/// }
/// fs::create_dir_all(test_origin_dir.join("dir1").join("dir2")).unwrap();
/// fs::create_dir_all(test_origin_dir.join("dir3")).unwrap();
/// let option = CopyOptions::new();
/// dir::copy("original_images", test_origin_dir.join("dir3"), &option).unwrap();
///
/// // It delete only dir1, dir2 but not dir3 because dir3 is not empty.
/// // And the function return Err.
/// assert!(delete_recursive(test_origin_dir).is_err());
///
/// assert!(test_origin_dir.join("dir3").is_dir());
/// assert!(!test_origin_dir.join("dir1").is_dir());
/// ```
pub fn delete_recursive<O: AsRef<Path>>(dir: O) -> Result<(), Box<dyn Error>> {
    if dir.as_ref().is_dir() {
        let mut is_file_exist = false;
        for content in read_dir(&dir)? {
            let content = content?;
            let content_path = content.path();
            if content_path.is_dir() {
                match delete_recursive(&content_path) {
                    Ok(_) => (),
                    Err(_) => is_file_exist = true,
                }
            } else if &content_path.file_name().unwrap().to_str().unwrap()[..1] != "." {
                is_file_exist = true;
            }
        }
        if !is_file_exist {
            remove_dir_all(dir).unwrap();
            Ok(())
        } else {
            Err(Box::new(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Directory is not empty!",
            )))
        }
    } else {
        Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a directory Error!",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::fs::File;
    use std::io::Write;

    const TEST_FILES: &'static [&str] = &["file1.txt", "file2.txt", "file3.txt", "file4.txt", "file5.txt"];

    /// Setup the test and return a tuple of root directory and file name vector.
    pub fn setup<T: AsRef<Path>>(test_name: T) -> (PathBuf, Vec<PathBuf>) {
        let dir_data = test_name.as_ref().to_path_buf();
        let files = vec![
            dir_data.join(TEST_FILES[0]),
            dir_data.join("dir1").join(TEST_FILES[1]),
            dir_data.join("dir1").join("dir2").join(TEST_FILES[2]),
            dir_data.join("dir1").join("dir2").join("dir3").join(TEST_FILES[3]),
            dir_data.join("dir1").join("dir2").join("dir3").join("dir4").join(TEST_FILES[4]),
        ];
        for file in &files {
            write_test_file(file).unwrap();
        }
        (dir_data, files)
    }
    
    /// Create dummy test files. 
    fn write_test_file<T: AsRef<Path>>(path: T) -> io::Result<()> {
        match &path.as_ref().parent() {
            Some(p) => fs::create_dir_all(&p).unwrap(),
            None => (),
        }
        write!(File::create(&path)?, "{}", "Hello world for ".to_owned() + path.as_ref().to_str().unwrap())?;
        Ok(())
    }

    fn cleanup<T: AsRef<Path>>(test_dir: T) {
        if test_dir.as_ref().is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
    }

    #[test]
    fn delete_recursive_test() {
        let (test_dir, test_files) = setup("delete_recursive_test_dir");
        if test_dir.is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
        assert!(delete_recursive(&test_dir).is_err());
        for test_file in test_files {
            assert!(!test_file.is_file());
        }
        cleanup(test_dir);
    }
}
