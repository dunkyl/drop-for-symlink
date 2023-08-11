use std::sync::atomic::*;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;
use windows::Win32::System::LibraryLoader::*;

use crate::extension_factory::DropForSymlinkFactory;
use crate::registry;
use crate::registry::*;

static INSTANCE: AtomicIsize = AtomicIsize::new(0);

const CLASS: GUID =
        GUID::from_u128(0x96D16936_E510_4EA4_8EE8_BC9C0BD7057B);
const CLASS_STR: &str = "{96D16936-E510-4EA4-8EE8-BC9C0BD7057B}";
const NAME: &str = "Drop for Symlink";

fn module_location() -> String {

    let mut filename_buf = vec![0u16; MAX_PATH as usize];

    // number of utf-16 elements of the filepath
    let chars =  unsafe {
        GetModuleFileNameW(HMODULE(INSTANCE.load(Ordering::Acquire)), &mut filename_buf)
    };
    filename_buf.truncate(chars as usize);

    String::from_utf16_lossy(&filename_buf)
}

fn registry_batch() -> RegBatch {
    registry!{ HKEY_LOCAL_MACHINE,
        + ["SOFTWARE\\Classes\\CLSID\\{CLASS_STR}"] {
            =(format!("{NAME} Factory"))
            + ["InProcServer32"] {
                =(module_location())
                "ThreadingModel"="Apartment"
            }
        }
        + ["SOFTWARE\\Classes\\Directory\\ShellEx\\DragDropHandlers\\{NAME}"] {
            =CLASS_STR
        }
        ["SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Shell Extensions\\Approved"] {
            CLASS_STR=NAME
        }
    }
}

#[no_mangle]
pub extern fn DllMain (inst: HMODULE, _reason: i32, _: isize) -> bool {
    INSTANCE.store(inst.0, Ordering::Release);
    true
}

#[no_mangle]
pub extern fn DllRegisterServer() -> HRESULT {
    if let Err(e) = registry_batch().apply() {
        registry_batch().unapply();
        e.code()
    } else {
        S_OK
    }
}

#[no_mangle]
pub extern fn DllGetClassObject(
    rClsid: *const GUID,
    rIid: *const GUID,
    ppv: *mut *const core::ffi::c_void
) -> HRESULT {
    let Some(rIid) = (unsafe { rIid.as_ref() }) else {
        return E_INVALIDARG
    };
    let Some(&rClsid) = (unsafe {rClsid.as_ref()}) else {
        return E_INVALIDARG
    };
    if rClsid != CLASS {
        return CLASS_E_CLASSNOTAVAILABLE
    }
    if ppv.is_null() {
        return E_INVALIDARG
    }

    let factory = DropForSymlinkFactory::default();

    unsafe {
        Into::<IUnknown>::into(factory).query(rIid, ppv)
    }
}

#[no_mangle]
pub extern fn DllUnregisterServer() -> HRESULT {
    registry_batch().unapply();
    S_OK
}

#[no_mangle]
pub extern fn DllInstall(_install: bool) -> HRESULT {
    S_OK
}