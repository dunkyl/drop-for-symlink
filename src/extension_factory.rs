use std::sync::atomic::*;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;

use crate::extension::DropForSymlinkExt;

static OBJ_COUNT: AtomicIsize = AtomicIsize::new(0);

#[implement(IClassFactory)]
pub struct DropForSymlinkFactory;

impl Default for DropForSymlinkFactory {
    fn default() -> Self {
        OBJ_COUNT.fetch_add(1, Ordering::AcqRel);
        DropForSymlinkFactory {  }
    }
}

impl Drop for DropForSymlinkFactory {
    fn drop(&mut self) {
        OBJ_COUNT.fetch_sub(1, Ordering::AcqRel);
    }
}

impl IClassFactory_Impl for DropForSymlinkFactory {
    fn CreateInstance(&self, pUnkOuter: Option<&IUnknown>, rIid: *const GUID, ppvObject: *mut *mut core::ffi::c_void) ->  Result<()> {

        if pUnkOuter.is_some() { return CLASS_E_NOAGGREGATION.ok() }

        let Some(rIid) = (unsafe { rIid.as_ref() }) else { return E_INVALIDARG.ok() };

        let inst = DropForSymlinkExt::default();

        let result = unsafe { IUnknown::from(inst).query(rIid, ppvObject as _) };

        result.ok()
    }

    fn LockServer(&self, _: BOOL) -> Result<()> { Ok(()) }
}

