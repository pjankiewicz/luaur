use alloc::string::String;

pub fn traverse_directory_rec_string_function_void_const_string_name(
    path: &str,
    callback: &dyn Fn(&str),
) -> bool {
    #[cfg(not(windows))]
    {
        use std::ffi::CStr;
        use std::fs::Metadata;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::MetadataExt;
        use std::path::Path;

        let path_obj = Path::new(path);
        if let Ok(entries) = std::fs::read_dir(path_obj) {
            for entry in entries.flatten() {
                let file_type = entry.file_type();
                if let Ok(ft) = file_type {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str == "." || name_str == ".." {
                        continue;
                    }

                    let full_path = path_obj.join(&file_name);
                    let path_str = full_path.to_string_lossy();

                    if ft.is_dir() {
                        traverse_directory_rec_string_function_void_const_string_name(
                            &path_str, callback,
                        );
                    } else if ft.is_file() {
                        callback(&path_str);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    #[cfg(windows)]
    {
        use crate::functions::from_utf_8::from_utf_8;
        use windows_sys::Win32::Storage::FileSystem::{
            FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_DIRECTORY, WIN32_FIND_DATAW,
        };

        let mut search_path = from_utf_8(path);
        // `search_path` is a UTF-16 buffer (Vec<u16>); append L"\\*" + NUL.
        search_path.extend_from_slice(&[b'\\' as u16, b'*' as u16, 0u16]);
        let mut find_data: WIN32_FIND_DATAW = unsafe { core::mem::zeroed() };
        let handle = unsafe { FindFirstFileW(search_path.as_ptr() as *const u16, &mut find_data) };

        if handle == -1isize as _ {
            return false;
        }

        loop {
            let name_ptr = find_data.cFileName.as_ptr();
            let name_len = unsafe {
                let mut len = 0;
                while *name_ptr.add(len) != 0 {
                    len += 1;
                }
                len
            };
            let name = String::from_utf16_lossy(unsafe {
                core::slice::from_raw_parts(name_ptr, name_len)
            });

            if name != "." && name != ".." {
                let mut full_path = String::from(path);
                full_path.push('\\');
                full_path.push_str(&name);

                if (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 {
                    traverse_directory_rec_string_function_void_const_string_name(
                        &full_path, callback,
                    );
                } else {
                    callback(&full_path);
                }
            }

            if unsafe { FindNextFileW(handle, &mut find_data) } == 0 {
                break;
            }
        }
        unsafe { FindClose(handle) };
        true
    }
}
