use std::path::PathBuf;

use file_system_proxy_traits::FileSystemProxy;
use file_types::Byteable;

use crate::file_analysis::file_system_proxy_traits::MetadataProxy;
use crate::file_analysis::file_types::DirectoryEntry;

pub(crate) mod file_system_proxy_traits;
pub(crate) mod file_types;

pub(crate) fn read_fs(current_dir: PathBuf, file_operations: &impl FileSystemProxy) -> DirectoryEntry {
    populate_tree(file_operations, current_dir, true)
}

fn populate_tree(file_operations: &impl FileSystemProxy, current_dir: PathBuf, is_root: bool) -> DirectoryEntry {
    match file_operations.read_dir(&current_dir) {
        Err(_e) => {
            // eprintln!("error in read_dir: {}", e);
            DirectoryEntry::Folder { path: current_dir, len: Byteable(0), entries: vec![], is_root, is_hidden: false }
        }
        Ok(read_dir) => {
            let mut len = 0_u64;
            let mut entries = vec![];
            for entry in read_dir {
                let entry = entry.expect("error in getting entry");
                //todo add check for hidden
                let dir = entry.path();
                match entry.file_type().expect("error getting file type").is_dir() {
                    true => {
                        let child = populate_tree(file_operations, dir, false);
                        len += child.len().0;
                        entries.push(child);
                    }
                    false => match file_operations.metadata(&dir) {
                        Ok(metadata) => {
                            len += metadata.len();
                            let len = Byteable(metadata.len());
                            let hidden = is_hidden(metadata);

                            entries.push(DirectoryEntry::new_file(len, dir, hidden));
                        }
                        Err(_e) => {
                            // eprintln!("error in metadata: {}", e);
                        }
                    },
                }
            }
            let hidden = file_operations.metadata(&current_dir).map(|m| is_hidden(m)).unwrap_or(true);

            let mut entry =
                DirectoryEntry::Folder { path: current_dir, len: Byteable(len), entries, is_root, is_hidden: hidden };
            entry.rollup();
            entry
        }
    }
}

fn is_hidden(metadata: Box<dyn MetadataProxy>) -> bool {
    (metadata.file_attributes() & 0000000000000000000000000000010) == 2
}

#[cfg(test)]
mod mock_utils;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::file_analysis::{mock_utils, read_fs};

    #[test]
    fn test_run_with_1_file() -> Result<(), Box<dyn Error>> {
        let (dir, mock_file_operations) = mock_utils::set_expect(0, 1)?;
        let entry = read_fs(dir, &mock_file_operations);
        assert_eq!(entry.len().0, 1024 * 1024_u64);
        if let Some(entries) = entry.entries() {
            let mut iter = entries.iter();
            let file = iter.next();
            assert_eq!(file.is_some(), true);
            let file = file.unwrap();
            assert_eq!(file.is_dir(), false);
            assert_eq!(file.len().0, 1024 * 1024_u64);
            assert_eq!(iter.next().is_none(), true);
            Ok(())
        } else {
            panic!("None should be some")
        }
    }

    #[test]
    fn test_run_with_1_directory() -> Result<(), Box<dyn Error>> {
        let (dir, mock_file_operations) = mock_utils::set_expect(1, 0)?;
        let entry = read_fs(dir, &mock_file_operations);
        assert_eq!(entry.len().0, 0_u64);
        if let Some(entries) = entry.entries() {
            let mut iter = entries.iter();
            let child = iter.next();
            assert_eq!(child.is_some(), true);
            let child = child.unwrap();
            assert_eq!(child.is_dir(), true);
            assert_eq!(child.name(), "test\\");
            assert_eq!(child.len().0, 0_u64);
            assert_eq!(iter.next().is_none(), true);
            Ok(())
        } else {
            panic!("None should be some")
        }
    }

    #[test]
    fn test_run_with_2_directory() -> Result<(), Box<dyn Error>> {
        let (dir, mock_file_operations) = mock_utils::set_expect(2, 0)?;
        let entry = read_fs(dir, &mock_file_operations);
        assert_eq!(entry.len().0, 0_u64);
        if let Some(entries) = entry.entries() {
            let mut iter = entries.iter();
            let child = iter.next();
            assert_eq!(child.is_some(), true);
            let entry = child.unwrap();
            assert_eq!(entry.name(), "test\\");
            assert_eq!(entry.is_dir(), true);

            let child = iter.next();
            assert_eq!(child.is_some(), true);
            let entry1 = child.unwrap();
            assert_eq!(entry1.name(), "test\\");
            assert_eq!(entry1.is_dir(), true);

            assert_eq!(iter.next().is_some(), false);
            Ok(())
        } else {
            panic!("None should be some")
        }
    }

    #[test]
    fn test_run_with_1_directory_and_1_file() -> Result<(), Box<dyn Error>> {
        let (dir, mock_file_operations) = mock_utils::set_expect(1, 1)?;
        let entry = read_fs(dir, &mock_file_operations);
        assert_eq!(entry.len().0, 1024 * 1024_u64);
        if let Some(entries) = entry.entries() {
            let mut iter = entries.iter();

            let file = iter.next();
            assert_eq!(file.is_some(), true);
            let entry = file.unwrap();
            assert_eq!(entry.is_dir(), false);
            assert_eq!(entry.len().0, 1024 * 1024_u64);

            let rollup = iter.next();
            assert_eq!(rollup.is_some(), true);
            let child = rollup.unwrap();
            assert_eq!(child.name(), "test\\");
            assert_eq!(child.is_dir(), true);
            assert_eq!(child.len().0, 0_u64);
            Ok(())
        } else {
            panic!("None should be some")
        }
    }

    #[test]
    fn test_run_with_2_directory_and_2_file() -> Result<(), Box<dyn Error>> {
        let (dir, mock_file_operations) = mock_utils::set_expect(2, 2)?;
        let entry = read_fs(dir, &mock_file_operations);
        assert_eq!(entry.len().0, 1024 * 1024_u64 * 2_u64);
        if let Some(entries) = entry.entries() {
            let mut iter = entries.iter();
            let file = iter.next();
            assert_eq!(file.is_some(), true);
            let entry = file.unwrap();
            assert_eq!(entry.is_dir(), false);
            assert_eq!(entry.len().0, 1024 * 1024_u64);
            let file = iter.next();
            assert_eq!(file.is_some(), true);
            let entry = file.unwrap();
            assert_eq!(entry.is_dir(), false);
            assert_eq!(entry.len().0, 1024 * 1024_u64);

            let child = iter.next();
            assert_eq!(child.is_some(), true);
            let child = child.unwrap();
            assert_eq!(child.is_dir(), true);
            assert_eq!(child.len().0, 0_u64);
            assert_eq!(child.name(), "test\\");

            let child = iter.next();
            assert_eq!(child.is_some(), true);
            let child = child.unwrap();
            assert_eq!(child.is_dir(), true);
            assert_eq!(child.len().0, 0_u64);
            assert_eq!(child.name(), "test\\");

            let child = iter.next();
            assert_eq!(child.is_some(), false);

            Ok(())
        } else {
            panic!("None should be some")
        }
    }
}
