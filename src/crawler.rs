//! Containing functions that return a list of files or folders.
//!
//! # Examples
//!
//! `get_file_list` example.
//! ```
//! use std::path::PathBuf;
//! use image_compressor::crawler::{get_file_list, get_dir_list};
//! let root = PathBuf::from("root");
//! get_file_list(&root);
//! get_dir_list(&root);
//! ```

use std::io;
use std::path::{Path, PathBuf};

/// Find all files in the root directory in a recursive way.
/// The hidden files started with `.` will be not inclused in result.
pub fn get_file_list<O: AsRef<Path>>(root: O) -> io::Result<Vec<PathBuf>> {
    let mut image_list: Vec<PathBuf> = Vec::new();
    let mut file_list: Vec<PathBuf> = root.as_ref().read_dir()?
        .map(|entry| entry.unwrap().path())
        .collect();
    let mut i = 0;
    loop {
        if i >= file_list.len(){break;}
        if file_list[i].is_dir(){
            for component in file_list[i].read_dir()? {
                file_list.push(component.unwrap().path());
            }
        }
        else if &file_list[i].file_name().unwrap().to_str().unwrap().chars().collect::<Vec<_>>()[0] != &'.' {
            image_list.push(file_list[i].to_path_buf());
        }
        i += 1;
    }

    Ok(image_list)
}

/// Get all directories list in the rood directory. Not recursive. 
pub fn get_dir_list<O: AsRef<Path>>(root: O) -> io::Result<Vec<PathBuf>> {
    let cur_list: Vec<PathBuf> = root.as_ref().read_dir()?
        .map(|entry| entry.unwrap().path())
        .collect();
    let dir_list = cur_list.iter()
        .filter(|p| p.is_dir())
        .map(|p| PathBuf::from(p.to_path_buf()))
        .collect::<Vec<_>>();

    Ok(dir_list)
}


#[cfg(test)]
mod tests {

    use super::*;
    use std::{io, fs};
    use std::path::{Path, PathBuf};

    struct DirData{
        origin: PathBuf,
        dest: PathBuf,
    }

    fn make_dir_data() -> DirData{
        DirData{origin: PathBuf::from("original_images"), dest: PathBuf::from("test_original_images")}
    }

    fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    fn copy_origin_for_test(){
        let DirData{origin, dest } = make_dir_data();
        if dest.is_dir(){
            fs::remove_dir_all(dest.as_path()).expect("cannot clear the test original directory!");
        }
        copy_dir_all(origin.as_path(), dest.as_path()).expect("cannot copy the original directory!");
    }

    fn remove_all_test_dir(){
        let DirData{origin: _, dest} = make_dir_data();
        if dest.is_dir() {
            fs::remove_dir_all(dest.as_path()).expect("cannot clear the test original directory!");
        }
        if dest.is_dir() {
            fs::remove_dir_all(dest.as_path()).expect("cannot clear the test destination directory!");
        }
    }

    /// Setup the test.
    fn setup(){
        let DirData{origin, dest: _ } = make_dir_data();
        assert!(origin.exists());
        assert!(!origin.is_file());
        remove_all_test_dir();
        copy_origin_for_test();
    }

    /// Clean up all tested data.
    fn cleanup(){
        remove_all_test_dir();
    }

    #[test]
    fn get_image_list_test(){
        setup();
        let mut test_vec = get_file_list(PathBuf::from("test_original_images").as_path()).unwrap();
        test_vec.sort();
        let mut expect_vec = vec![PathBuf::from("test_original_images/file2.jpg"),
                              PathBuf::from("test_original_images/file1.png"),
                              PathBuf::from("test_original_images/dir1/file3.png"),
                              PathBuf::from("test_original_images/file4.jpg"),
                              PathBuf::from("test_original_images/dir1/file5.webp"),
                              PathBuf::from("test_original_images/file7.txt"),
                              PathBuf::from("test_original_images/dir1/dir2/file6.webp")];
        expect_vec.sort();

        assert_eq!(test_vec, expect_vec);
        cleanup();
    }
}