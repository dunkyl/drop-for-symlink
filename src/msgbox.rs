use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::winstr::WinStr;

pub fn msgBox(title: impl Into<WinStr>, text: impl Into<WinStr>, ty: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe {
        MessageBoxW(HWND::default(), text.into().get_pcwstr(), title.into().get_pcwstr(), ty)
    }
}