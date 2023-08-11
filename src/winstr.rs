use std::{fmt::{Display, Debug}, sync::Arc};

use windows::core::PCWSTR;

#[derive(Clone)]
pub struct WinStr {
    data: Arc<[u16]>
}

impl WinStr {
    pub fn bytes_slice(&self) -> &[u8] {
        unsafe {
            self.data.align_to().1 
        }
    }

    pub fn from_buffer(buf: Vec<u16>, sz: usize) -> Self {
        WinStr {
            data: buf.into_iter().take(sz).collect()
        }
    }

    pub fn from_slice(slice: &[u16]) -> Self {
        WinStr {
            data: slice.into()
        }
    }

    fn get_string(&self) -> String {
        let null_terminated = String::from_utf16_lossy(&self.data);
        null_terminated.trim_end_matches('\0').into()
    }

    pub fn get_pcwstr(&self) -> PCWSTR {
        PCWSTR(self.data.as_ptr())
    }

    // pub fn get_pwstr(&mut self) -> PWSTR {
    //     PWSTR(self.data.as_mut_ptr())
    // }

    pub fn len_with_terminator(&self) -> usize {
        self.data.len()
    }

    pub fn pwcstr_or_null(opt_string: Option<&Self>) -> PCWSTR {
        opt_string.map(Self::get_pcwstr).unwrap_or(PCWSTR::null())
    }
}

impl<T> From<T> for WinStr
where T: AsRef<str> {
    fn from(value: T) -> Self {
        let str = value.as_ref();
        WinStr {
            data: str.encode_utf16().chain(std::iter::once(0)).collect()
        } 
    }
}

impl From<WinStr> for String {
    fn from(value: WinStr) -> Self {
        value.get_string()
    }
}

impl Display for WinStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.get_string(), f)
    }
}

impl Debug for WinStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("w")?;
        std::fmt::Debug::fmt(&self.get_string(), f)
    }
}