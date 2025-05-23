use core::{marker::PhantomData, slice};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FFIStr<'a> {
    ptr: *const u8,
    len: usize,
    phantom: PhantomData<&'a str>,
}

impl<'a> From<&'a str> for FFIStr<'a> {
    fn from(value: &'a str) -> Self {
        FFIStr { ptr: value.as_ptr(), len: value.len(), phantom: PhantomData }
    }
}

impl<'a> Into<&'a str> for FFIStr<'a> {
    fn into(self) -> &'a str {
        // SAFETY: SHOULD BE SAFE
        str::from_utf8(unsafe { slice::from_raw_parts(self.ptr, self.len) }).unwrap_or("malformed_ffi_str")
    }
}
