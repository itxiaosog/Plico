//! 平台相关的薄封装。所有 unsafe 都关在这个文件里，上层拿到的都是安全接口。

/// 剪贴板序号。Windows 上每次剪贴板内容变化都会自增，比对比内容更快也更省。
#[cfg(windows)]
pub fn clipboard_sequence() -> u32 {
    unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() }
}

#[cfg(not(windows))]
pub fn clipboard_sequence() -> u32 {
    0
}

/// 当前前台窗口的 (可执行文件名, 窗口标题)。用于 F1 的来源记录。
/// 任何一步失败都返回 None —— 来源信息是锦上添花，不该影响入库。
#[cfg(windows)]
pub fn foreground_app() -> (Option<String>, Option<String>) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return (None, None);
        }

        // 窗口标题
        let title = {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; len as usize + 1];
                let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
                if n > 0 {
                    Some(String::from_utf16_lossy(&buf[..n as usize]))
                } else {
                    None
                }
            } else {
                None
            }
        };

        // 进程名
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return (None, title);
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return (None, title);
        }

        let mut path_buf = vec![0u16; 512];
        let mut size = path_buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, path_buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if ok == 0 {
            return (None, title);
        }

        let full = String::from_utf16_lossy(&path_buf[..size as usize]);
        let exe = full
            .rsplit(['\\', '/'])
            .next()
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        (exe, title)
    }
}

#[cfg(not(windows))]
pub fn foreground_app() -> (Option<String>, Option<String>) {
    (None, None)
}
