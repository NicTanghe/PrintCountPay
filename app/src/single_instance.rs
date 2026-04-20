#[cfg(target_os = "windows")]
mod platform {
    use std::error::Error;
    use std::ffi::c_void;
    use std::fmt;
    use std::ptr::{null, null_mut};

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const INSTANCE_MUTEX_NAME: &str = "Global\\PrintCountPay.SingleInstance";
    const MB_ICONERROR: u32 = 0x00000010;
    const MB_ICONINFORMATION: u32 = 0x00000040;
    const MB_OK: u32 = 0x00000000;

    type Bool = i32;
    type Handle = *mut c_void;
    type Hwnd = *mut c_void;
    type SecurityAttributes = c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *const SecurityAttributes,
            b_initial_owner: Bool,
            lp_name: *const u16,
        ) -> Handle;
        fn CloseHandle(h_object: Handle) -> Bool;
        fn GetLastError() -> u32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: Hwnd, lp_text: *const u16, lp_caption: *const u16, u_type: u32)
        -> i32;
    }

    #[derive(Debug)]
    pub(crate) struct SingleInstanceError {
        last_error: u32,
    }

    impl SingleInstanceError {
        fn new(last_error: u32) -> Self {
            Self { last_error }
        }
    }

    impl fmt::Display for SingleInstanceError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "single-instance guard failed with Windows error {}",
                self.last_error
            )
        }
    }

    impl Error for SingleInstanceError {}

    pub(crate) struct SingleInstanceGuard {
        handle: Handle,
    }

    impl SingleInstanceGuard {
        pub(crate) fn acquire() -> Result<Option<Self>, SingleInstanceError> {
            let name = wide_null(INSTANCE_MUTEX_NAME);
            let handle = unsafe { CreateMutexW(null(), 0, name.as_ptr()) };
            if handle.is_null() {
                return Err(SingleInstanceError::new(unsafe { GetLastError() }));
            }

            let last_error = unsafe { GetLastError() };
            if last_error == ERROR_ALREADY_EXISTS {
                unsafe {
                    CloseHandle(handle);
                }
                return Ok(None);
            }

            Ok(Some(Self { handle }))
        }
    }

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    pub(crate) fn show_already_running_message() {
        show_message(
            "PrintCountPay is already running. Close the current window before launching it again.",
            "PrintCountPay",
            MB_OK | MB_ICONINFORMATION,
        );
    }

    pub(crate) fn show_startup_error(message: &str) {
        show_message(
            message,
            "PrintCountPay startup failed",
            MB_OK | MB_ICONERROR,
        );
    }

    fn show_message(message: &str, title: &str, flags: u32) {
        let message = wide_null(message);
        let title = wide_null(title);
        unsafe {
            MessageBoxW(null_mut(), message.as_ptr(), title.as_ptr(), flags);
        }
    }

    fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use std::convert::Infallible;

    pub(crate) type SingleInstanceError = Infallible;

    pub(crate) struct SingleInstanceGuard;

    impl SingleInstanceGuard {
        pub(crate) fn acquire() -> Result<Option<Self>, SingleInstanceError> {
            Ok(Some(Self))
        }
    }

    pub(crate) fn show_already_running_message() {}

    pub(crate) fn show_startup_error(_message: &str) {}
}

pub(crate) use platform::{SingleInstanceGuard, show_already_running_message, show_startup_error};
