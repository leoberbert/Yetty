//! The Windows "Open", "Save as" and "Browse for folder" dialogs, called directly (fast: no helper process).

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    #[repr(C)]
    pub struct OpenFileNameW {
        pub l_struct_size: u32,
        pub hwnd_owner: isize,
        pub hinstance: isize,
        pub filter: *const u16,
        pub custom_filter: *mut u16,
        pub max_custom_filter: u32,
        pub filter_index: u32,
        pub file: *mut u16,
        pub max_file: u32,
        pub file_title: *mut u16,
        pub max_file_title: u32,
        pub initial_dir: *const u16,
        pub title: *const u16,
        pub flags: u32,
        pub file_offset: u16,
        pub file_extension: u16,
        pub def_ext: *const u16,
        pub cust_data: isize,
        pub hook: isize,
        pub template_name: *const u16,
        pub reserved: *mut c_void,
        pub reserved_dw: u32,
        pub flags_ex: u32,
    }

    #[repr(C)]
    pub struct BrowseInfoW {
        pub hwnd_owner: isize,
        pub pidl_root: *const c_void,
        pub display_name: *mut u16,
        pub title: *const u16,
        pub flags: u32,
        pub callback: isize,
        pub lparam: isize,
        pub image: i32,
    }

    #[link(name = "comdlg32")]
    extern "system" {
        pub fn GetOpenFileNameW(o: *mut OpenFileNameW) -> i32;
        pub fn GetSaveFileNameW(o: *mut OpenFileNameW) -> i32;
    }
    #[link(name = "shell32")]
    extern "system" {
        pub fn SHBrowseForFolderW(b: *mut BrowseInfoW) -> *mut c_void;
        pub fn SHGetPathFromIDListW(pidl: *const c_void, path: *mut u16) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        pub fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
        pub fn CoTaskMemFree(p: *mut c_void);
    }
    #[link(name = "user32")]
    extern "system" {
        pub fn GetForegroundWindow() -> isize;
    }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn file_dialog(
    title: &str,
    start_dir: Option<&str>,
    save: bool,
    filter_spec: &str,
) -> Option<String> {
    use win::*;
    let title_w = wide(title);
    let dir_w = start_dir
        .filter(|d| std::path::Path::new(d).is_dir())
        .map(wide);
    // "All files (*.*)" filter: pairs of NUL-terminated strings, ended by an extra NUL
    let filter: Vec<u16> = filter_spec.encode_utf16().collect();
    let mut buf = vec![0u16; 4096];
    let mut o = OpenFileNameW {
        l_struct_size: std::mem::size_of::<OpenFileNameW>() as u32,
        hwnd_owner: unsafe { GetForegroundWindow() },
        hinstance: 0,
        filter: filter.as_ptr(),
        custom_filter: std::ptr::null_mut(),
        max_custom_filter: 0,
        filter_index: 1,
        file: buf.as_mut_ptr(),
        max_file: buf.len() as u32,
        file_title: std::ptr::null_mut(),
        max_file_title: 0,
        initial_dir: dir_w
            .as_ref()
            .map(|d| d.as_ptr())
            .unwrap_or(std::ptr::null()),
        title: title_w.as_ptr(),
        // OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST (+ OFN_FILEMUSTEXIST when opening)
        flags: 0x0008_0000 | 0x0000_0008 | 0x0000_0800 | if save { 0 } else { 0x0000_1000 },
        file_offset: 0,
        file_extension: 0,
        def_ext: std::ptr::null(),
        cust_data: 0,
        hook: 0,
        template_name: std::ptr::null(),
        reserved: std::ptr::null_mut(),
        reserved_dw: 0,
        flags_ex: 0,
    };
    let ok = unsafe {
        if save {
            GetSaveFileNameW(&mut o)
        } else {
            GetOpenFileNameW(&mut o)
        }
    };
    if ok == 0 {
        return None;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

pub fn open_file(title: &str, start_dir: Option<&str>) -> Option<String> {
    #[cfg(windows)]
    {
        file_dialog(title, start_dir, false, "All files (*.*)\0*.*\0\0")
    }
    #[cfg(not(windows))]
    {
        let _ = (title, start_dir);
        None
    }
}

/// "Select Script to Run": the Windows Open dialog with the script file types first, as in the original.
pub fn open_script(title: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let types = "*.vbs;*.vbe;*.js;*.pl;*.py;*.ps1;*.txt";
        let spec = format!("Script Files ({types})\0{types}\0All Files (*.*)\0*.*\0\0");
        file_dialog(title, None, false, &spec)
    }
    #[cfg(not(windows))]
    {
        let _ = title;
        None
    }
}

pub fn save_file(title: &str) -> Option<String> {
    #[cfg(windows)]
    {
        file_dialog(title, None, true, "All files (*.*)\0*.*\0\0")
    }
    #[cfg(not(windows))]
    {
        let _ = title;
        None
    }
}

pub fn pick_folder(title: &str) -> Option<String> {
    #[cfg(windows)]
    unsafe {
        use win::*;
        CoInitializeEx(std::ptr::null_mut(), 2); // apartment threaded; already initialised is fine
        let title_w = wide(title);
        let mut name = vec![0u16; 260];
        let mut b = BrowseInfoW {
            hwnd_owner: GetForegroundWindow(),
            pidl_root: std::ptr::null(),
            display_name: name.as_mut_ptr(),
            title: title_w.as_ptr(),
            flags: 0x0001 | 0x0040, // only file-system folders, new dialog style
            callback: 0,
            lparam: 0,
            image: 0,
        };
        let pidl = SHBrowseForFolderW(&mut b);
        if pidl.is_null() {
            return None;
        }
        let mut path = vec![0u16; 1024];
        let ok = SHGetPathFromIDListW(pidl, path.as_mut_ptr());
        CoTaskMemFree(pidl);
        if ok == 0 {
            return None;
        }
        let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
        Some(String::from_utf16_lossy(&path[..len]))
    }
    #[cfg(not(windows))]
    {
        let _ = title;
        None
    }
}
