//! macOS draws the **first menu’s title** from the process / bundle identity, not from the
//! [`muda::Submenu`] label alone. It also builds “About …”, “Hide …”, “Quit …” from
//! `NSRunningApplication.localizedName` when no custom text is passed — which is the binary
//! name (`ui`) for `cargo run`.  
//! Call [`set`] as the first step in `main` so the menu bar and those items match
//! [`crate::app_name::APP_DISPLAY_NAME`].

use std::ffi::CString;
use std::os::raw::c_char;

use cocoa::base::nil;
use cocoa::foundation::NSString;
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};

/// Set the name shown in the menu bar and in standard app-menu items.
pub fn set(name: &str) {
    ns_process_info_set_process_name(name);
    let _ = cps_set_process_name(name);
}

fn ns_process_info_set_process_name(name: &str) {
    unsafe {
        let Some(cls) = Class::get("NSProcessInfo") else {
            return;
        };
        let proc_info: *mut Object = msg_send![cls, processInfo];
        if proc_info.is_null() {
            return;
        }
        let ns_name = NSString::alloc(nil).init_str(name);
        let _: () = msg_send![proc_info, setProcessName: ns_name];
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ProcessSerialNumber {
    high: u32,
    low: u32,
}

type OsStatus = i32;

fn cps_set_process_name(name: &str) -> bool {
    let Ok(c_name) = CString::new(name) else {
        return false;
    };
    unsafe {
        let lib = libc::dlopen(
            c"/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices"
                .as_ptr()
                .cast(),
            libc::RTLD_LAZY,
        );
        if lib.is_null() {
            return false;
        }
        type GetCurrentProcess = unsafe extern "C" fn(*mut ProcessSerialNumber) -> OsStatus;
        type CpsSetProcessName =
            unsafe extern "C" fn(*const ProcessSerialNumber, *const c_char) -> OsStatus;
        let get_current = libc::dlsym(lib, c"GetCurrentProcess".as_ptr().cast());
        let set_name = libc::dlsym(lib, c"CPSSetProcessName".as_ptr().cast());
        if get_current.is_null() || set_name.is_null() {
            libc::dlclose(lib);
            return false;
        }
        let get_current: GetCurrentProcess = std::mem::transmute(get_current);
        let set_name: CpsSetProcessName = std::mem::transmute(set_name);
        let mut psn = ProcessSerialNumber { high: 0, low: 0 };
        if get_current(&mut psn) != 0 {
            libc::dlclose(lib);
            return false;
        }
        let status = set_name(&psn, c_name.as_ptr());
        libc::dlclose(lib);
        status == 0
    }
}
