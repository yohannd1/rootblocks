use std::ffi::{CString, NulError};
use std::marker::PhantomData;
use std::os::raw::c_int;
use std::ptr;

// TODO: improve error handling
// TODO: make this into a bigger module

mod internal {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub struct XDisplay {
    /// The actual X display struct from the headers.
    ///
    /// Although this value is allowed to be null (since it is a C pointer), it never is because there are safe guards
    /// on the struct creation to prevent this.
    display_ptr: *mut internal::Display,
}

impl XDisplay {
    /// Attempts to open an X display and, if successful, returns it.
    pub fn open() -> Option<Self> {
        let display_ptr = unsafe { internal::XOpenDisplay(ptr::null()) };

        if display_ptr.is_null() {
            None
        } else {
            Some(Self { display_ptr })
        }
    }

    pub fn default_screen(&self) -> Screen<'_> {
        Screen {
            display_ptr: self.display_ptr,
            id: 
unsafe { internal::utils_x_default_screen(self.display_ptr) },
            _phantom: PhantomData,
        }
    }
}

impl Drop for XDisplay {
    fn drop(&mut self) {
        unsafe {
            internal::XCloseDisplay(self.display_ptr);
        }
    }
}

#[derive(Clone)]
pub struct Screen<'a> {
    display_ptr: *mut internal::Display,
    id: c_int,
    _phantom: PhantomData<&'a ()>,
}

impl Screen<'_> {
    pub fn root_window(&self) -> Window<'_> {
        Window {
            display_ptr: self.display_ptr,
            inner: unsafe { internal::utils_x_root_window(self.display_ptr, self.id) },
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct Window<'a> {
    display_ptr: *mut internal::Display,
    inner: internal::Window,
    _phantom: PhantomData<&'a ()>,
}

impl Window<'_> {
    pub fn set_name(&mut self, name: &str) -> Result<(), NulError> {
        let name = CString::new(name)?;

        unsafe {
            internal::XStoreName(
                self.display_ptr,
                self.inner,
                name.as_bytes_with_nul().as_ptr() as *const i8,
            );
        }

        Ok(())
    }
}
