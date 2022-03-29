use std::error::Error;
use std::fs::{read_dir, remove_dir_all};
use std::path::Path;
use std::io;

pub fn delete_recursive<O: AsRef<Path>>(dir: O)->Result<(), Box<dyn Error>>{
    if dir.as_ref().is_dir(){
        for content in read_dir(&dir)?{
            let content = content?;
            let content_path = content.path();
            if content_path.is_dir(){
                delete_recursive(&content_path)?
            }
            else if &content_path.file_name().unwrap().to_str().unwrap()[..1] != "."{
                return Err(Box::new(io::Error::new(io::ErrorKind::AlreadyExists, "Directory is not empty!")));
            }
        }
        remove_dir_all(dir).unwrap();
    }

    return Ok(());
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
        //let option = CopyOptions::new();
        //dir::copy("original_images", test_origin_dir, &option).unwrap();
        match delete_recursive(test_origin_dir){
            Ok(_) => (),
            Err(_) => println!("qegqegqegqeg"),
        };
        assert!(!test_origin_dir.is_dir());

    }
}