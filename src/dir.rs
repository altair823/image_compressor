//! Functions related to directory manipulation.
//! 

use std::error::Error;
use std::fs::{read_dir, remove_dir_all};
use std::path::Path;
use std::io;


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
pub fn delete_recursive<O: AsRef<Path>>(dir: O)->Result<(), Box<dyn Error>>{
    if dir.as_ref().is_dir(){
        let mut is_file_exist = false;
        for content in read_dir(&dir)?{
            let content = content?;
            let content_path = content.path();
            if content_path.is_dir(){
                match delete_recursive(&content_path){
                    Ok(_) => (),
                    Err(_) => is_file_exist = true,
                }
            }
            else if &content_path.file_name().unwrap().to_str().unwrap()[..1] != "."{
                is_file_exist = true;
            }
        }
        if is_file_exist == false{
            remove_dir_all(dir).unwrap();
            return Ok(());
        } 
        else {
            return Err(
                Box::new(
                    io::Error::new(
                        io::ErrorKind::AlreadyExists, "Directory is not empty!"
                    )
                )
            );
        }
    }
    else {
        Err(
            Box::new(
                io::Error::new(
                    io::ErrorKind::InvalidInput, "Not a directory Error!"
                )
            )
        )
    }
}


#[cfg(test)]
mod tests{
    use super::*;
    use std::fs;
    use fs_extra::dir;
    use fs_extra::dir::CopyOptions;

    #[test]
    fn delete_recursive_test(){
        let test_origin_dir = Path::new("test_origin");
        if test_origin_dir.is_dir(){
            fs::remove_dir_all(test_origin_dir).unwrap();
        }
        fs::create_dir_all(test_origin_dir.join("dir1").join("dir2")).unwrap();
        fs::create_dir_all(test_origin_dir.join("dir3")).unwrap();
        let option = CopyOptions::new();
        dir::copy("original_images", test_origin_dir.join("dir3"), &option).unwrap();
        assert!(delete_recursive(test_origin_dir).is_err());
        assert!(test_origin_dir.join("dir3").is_dir());
        assert!(!test_origin_dir.join("dir1").is_dir());

        fs::remove_dir_all(test_origin_dir).unwrap();
    }
}