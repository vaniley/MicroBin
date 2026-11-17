use std::{ffi::OsStr, mem, os::windows::ffi::OsStrExt, ptr};

use windows_sys::Win32::{
    Foundation::{GetLastError, HWND, LRESULT, POINT},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{NIF_ICON, NIF_MESSAGE, NIM_ADD},
        WindowsAndMessaging::{
            CreatePopupMenu, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos,
            GetMenuItemID, GetMessageW, LoadIconW, PostQuitMessage, RegisterClassW,
            RegisterWindowMessageW, SetForegroundWindow, SetMenuInfo, TrackPopupMenu,
            TranslateMessage, CW_USEDEFAULT, HICON, IDI_APPLICATION, MENUINFO, MIM_APPLYTOSUBMENUS,
            MIM_STYLE, MNS_NOTIFYBYPOS, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_LEFTBUTTON,
            WM_LBUTTONUP, WM_MENUCOMMAND, WM_QUIT, WM_RBUTTONUP, WM_USER, WNDCLASSW,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use {super::*, crate::TIError};

pub(crate) fn to_wstring(str: &str) -> Vec<u16> {
    OsStr::new(str)
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>()
}

pub(crate) unsafe fn get_win_os_error(msg: &str) -> TIError {
    TIError::new_with_location(
        format!("{}: {}", msg, GetLastError()),
        std::file!(),
        std::line!(),
    )
}

pub(crate) unsafe extern "system" fn window_proc(
    h_wnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if msg == WM_MENUCOMMAND {
        WININFO_STASH.with(|stash| {
            let stash = stash.borrow();
            let stash = stash.as_ref();
            if let Some(stash) = stash {
                let menu_id = GetMenuItemID(stash.info.hmenu, w_param as i32) as i32;
                if menu_id != -1 {
                    stash.tx.send(WindowsTrayEvent(menu_id as u32)).ok();
                }
            }
        });
    }

    if msg == WM_USER + 1 && (l_param as u32 == WM_LBUTTONUP || l_param as u32 == WM_RBUTTONUP) {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) == 0 {
            return 1;
        }

        SetForegroundWindow(h_wnd);

        WININFO_STASH.with(|stash| {
            let stash = stash.borrow();
            let stash = stash.as_ref();
            if let Some(stash) = stash {
                TrackPopupMenu(
                    stash.info.hmenu,
                    TPM_LEFTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                    point.x,
                    point.y,
                    0,
                    h_wnd,
                    ptr::null(),
                );
            }
        });
    }

    // If windows explorer restarts and we need to recreate the tray icon
    let restore = WININFO_STASH.with(|stash| {
        let stash = stash.borrow();
        stash
            .as_ref()
            .and_then(|data| data.info.notification_for_message(msg))
    });
    if let Some(nid) = restore {
        // Keep the configured raw icon alive in the caller. Do not reload a
        // generic resource, lose the tooltip, or hold a lock across a Shell call.
        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            println!("Error adding menu icon");
        }
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
    }

    DefWindowProcW(h_wnd, msg, w_param, l_param)
}

pub(crate) unsafe fn init_window() -> Result<WindowInfo, TIError> {
    let taskbar_created = RegisterWindowMessageW(to_wstring("TaskbarCreated").as_ptr());
    if taskbar_created == 0 {
        return Err(get_win_os_error("Error registering TaskbarCreated"));
    }
    let hmodule = GetModuleHandleW(ptr::null());
    if hmodule == 0 {
        return Err(get_win_os_error("Error getting module handle"));
    }

    let class_name = to_wstring("my_window");

    let mut wnd = unsafe { mem::zeroed::<WNDCLASSW>() };
    wnd.lpfnWndProc = Some(window_proc);
    wnd.lpszClassName = class_name.as_ptr();

    RegisterClassW(&wnd);

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        to_wstring("rust_systray_window").as_ptr(),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        0,
        0,
        0,
        ptr::null(),
    );
    if hwnd == 0 {
        return Err(get_win_os_error("Error creating window"));
    }

    let icon: HICON = unsafe {
        let mut handle = LoadIconW(
            GetModuleHandleW(std::ptr::null()),
            to_wstring("tray-default").as_ptr(),
        );
        if handle == 0 {
            handle = LoadIconW(0, IDI_APPLICATION);
        }
        if handle == 0 {
            return Err(get_win_os_error("Error setting icon from resource"));
        }
        handle as HICON
    };

    let mut nid = unsafe { mem::zeroed::<NOTIFYICONDATAW>() };
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON;
    nid.hIcon = icon;
    nid.uCallbackMessage = WM_USER + 1;
    if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
        return Err(get_win_os_error("Error adding menu icon"));
    }

    // Setup menu
    let mut info = unsafe { mem::zeroed::<MENUINFO>() };
    info.cbSize = mem::size_of::<MENUINFO>() as u32;
    info.fMask = MIM_APPLYTOSUBMENUS | MIM_STYLE;
    info.dwStyle = MNS_NOTIFYBYPOS;
    let hmenu = CreatePopupMenu();
    if hmenu == 0 {
        return Err(get_win_os_error("Error creating popup menu"));
    }
    if SetMenuInfo(hmenu, &info) == 0 {
        return Err(get_win_os_error("Error setting up menu"));
    }

    Ok(WindowInfo {
        hwnd,
        hmenu,
        hmodule,
        notification: Arc::new(Mutex::new(nid)),
        taskbar_created,
    })
}

pub(crate) unsafe fn run_loop() {
    // Run message loop
    let mut msg = unsafe { mem::zeroed::<MSG>() };
    loop {
        GetMessageW(&mut msg, 0, 0, 0);
        if msg.message == WM_QUIT {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
