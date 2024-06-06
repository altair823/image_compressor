//! Containing functions that return a list of files or folders.
//!
//! # Examples
//!
//! `get_file_list` example.
//! ```
//! use std::path::PathBuf;
//! use image_compressor::crawler::get_file_list;
//! let root = PathBuf::from("root");
//! get_file_list(&root);
//! ```

use std::io;
use std::path::{Path, PathBuf};

/// Find all files in the root directory in a recursive way.
/// The hidden files started with `.` will be not included in result.
pub fn get_file_list<O: AsRef<Path>>(root: O) -> io::Result<Vec<PathBuf>> {
    let mut image_list: Vec<PathBuf> = Vec::new();
    let mut file_list: Vec<PathBuf> = root
        .as_ref()
        .read_dir()?
        .map(|entry| entry.unwrap().path())
        .collect();
    let mut i = 0;
    loop {
        if i >= file_list.len() {
            break;
        }
        if file_list[i].is_dir() {
            for component in file_list[i].read_dir()? {
                file_list.push(component.unwrap().path());
            }
        } else if file_list[i]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .collect::<Vec<_>>()[0]
            != '.'
        {
            image_list.push(file_list[i].to_path_buf());
        }
        i += 1;
    }

    Ok(image_list)
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::{fs, io};

    const CRAWLER_TEST_FILES: &'static [&str] = &[
        "file1.txt",
        "file2.txt",
        "file3.txt",
        "file4.txt",
        "file5.txt",
    ];

    /// Create dummy test files.
    fn write_test_file<T: AsRef<Path>>(path: T) -> io::Result<()> {
        match &path.as_ref().parent() {
            Some(p) => fs::create_dir_all(&p).unwrap(),
            None => (),
        }
        write!(
            File::create(&path)?,
            "{}",
            "Hello world for ".to_owned() + path.as_ref().to_str().unwrap()
        )?;
        Ok(())
    }

    /// Setup the test and return a tuple of root directory and file name vector.
    pub fn setup<T: AsRef<Path>>(test_name: T) -> (PathBuf, Vec<PathBuf>) {
        let dir_data = test_name.as_ref().to_path_buf();
        let files = vec![
            dir_data.join(CRAWLER_TEST_FILES[0]),
            dir_data.join("dir1").join(CRAWLER_TEST_FILES[1]),
            dir_data
                .join("dir1")
                .join("dir2")
                .join(CRAWLER_TEST_FILES[2]),
            dir_data
                .join("dir1")
                .join("dir2")
                .join("dir3")
                .join(CRAWLER_TEST_FILES[3]),
            dir_data
                .join("dir1")
                .join("dir2")
                .join("dir3")
                .join("dir4")
                .join(CRAWLER_TEST_FILES[4]),
        ];
        for file in &files {
            write_test_file(file).unwrap();
        }
        (dir_data, files)
    }

    fn cleanup<T: AsRef<Path>>(test_dir: T) {
        if test_dir.as_ref().is_dir() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
    }

    #[test]
    fn get_file_list_test() {
        let (test_dir, mut expected_vec) = setup("get_file_list_test_dir");
        let mut test_vec = get_file_list(&test_dir).unwrap();
        test_vec.sort();
        expected_vec.sort();
        assert_eq!(test_vec, expected_vec);
        cleanup(test_dir);
    }
}
