use std::ops::DerefMut;
use std::sync::Mutex;

use regex::Captures;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Ole::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::Shell::Common::*;
use windows::Win32::UI::WindowsAndMessaging::*;


#[implement(IShellExtInit, IContextMenu)]
#[derive(Default)]
pub struct DropForSymlinkExt {
    file_paths: Mutex<Vec<String>>,
    folder: Mutex<String>
}

impl IShellExtInit_Impl for DropForSymlinkExt {
    fn Initialize(&self, pIdlFolder: *const ITEMIDLIST, pdtObj: Option<&IDataObject>, _hkeyProgid: HKEY) -> Result<()> {

        let mut path_buf = [0u16; 260];

        if pIdlFolder.is_null() {
            return E_INVALIDARG.ok();
        }
        
        unsafe {
            SHGetPathFromIDListW(pIdlFolder, &mut path_buf);
            let s = String::from_utf16_lossy(path_buf.split(|x|*x==0).next().unwrap());
            *self.folder.lock().unwrap() = s;
        }

        let Some(&obj) = pdtObj.as_ref() else { return E_UNEXPECTED.ok() };

        let format = FORMATETC {
            cfFormat: CF_HDROP.0,
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
            ..Default::default()
        };
        unsafe {
            obj.QueryGetData(&format).ok()?;

            let mut medium = obj.GetData(&format)?;

            // union case assumed by calling site
            let hGlobal: HDROP = std::mem::transmute(medium.Anonymous.hGlobal);
            let file_index = 0xFFFFFFFF; // magic value for querying file count
            
            let count = DragQueryFileW(hGlobal, file_index, Some(&mut path_buf));

            if count == 0 {
                return E_UNEXPECTED.ok()
            }
            let mut file_names = self.file_paths.lock().unwrap();

            for i in 0..count {
                DragQueryFileW(hGlobal, i, Some(&mut path_buf));
                file_names.push(String::from_utf16_lossy(path_buf.split(|x|*x==0).next().unwrap()))
            }

            ReleaseStgMedium(&mut medium);

        }
        
        S_OK.ok()
    }
}

impl IContextMenu_Impl for DropForSymlinkExt {
    fn QueryContextMenu(&self, hMenu: HMENU, indexMenu: u32, idCmdFirst: u32, _idCmdLast: u32, uFlags: u32) -> Result<()> {

        if (CMF_DEFAULTONLY & uFlags) != 0 {
            return S_OK.ok();
        }
        
        unsafe {

            let mut text: String = "Create Symlink".into();

            let file_paths = self.file_paths.lock().unwrap();

            if file_paths.len() > 1 {
                text += "s" // plural
            }

            let mut bytes: Vec<_> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let my_item = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING | MIIM_ID,
                wID: idCmdFirst, // idCmdFirst <= wID <=  idCmdLast
                dwTypeData: PWSTR(bytes.as_mut_ptr()),
                ..Default::default()
            };
            let success = InsertMenuItemW(
                hMenu,
                indexMenu,
                true,
                (&my_item) as * const _
            );

            if !success.as_bool() {
                return Err(Error::from_win32())
            }

            let max_wID = my_item.wID as i32;
            Err(HRESULT(max_wID - idCmdFirst as i32 + 1).into())
        }
    }

    fn InvokeCommand(&self, lpCmi: *const CMINVOKECOMMANDINFO) -> Result<()> {
        let Some(&lpCmi) = (unsafe { lpCmi.as_ref() }) else {
            return E_UNEXPECTED.ok()
        };

        if (lpCmi.lpVerb.as_ptr() as usize) & 0xFF != 0 {
            return E_FAIL.ok();
        }

        let mut folder = self.folder.lock().unwrap();
        let mut file_paths = self.file_paths.lock().unwrap();

        let folder_path: std::path::PathBuf = std::mem::take(folder.deref_mut()).into();

        let renamed_file_re = regex::Regex::new(r" \((\d+)\)$").unwrap();

        for file_path in std::mem::take::<Vec<_>>(file_paths.as_mut()) {
            let file_path: std::path::PathBuf = file_path.into();
            let Some(name) = file_path.file_name() else { continue; };

            let mut symlink_path = folder_path.join(name);

            if symlink_path.to_str().is_none() {
                return E_UNEXPECTED.ok();
            }

            if symlink_path.metadata().is_ok() {
                if crate::msgbox::msgBox(
                    "Drop for Symlink",
                    format!("Link path already exists:\n{}\nContinue renamed?", 
                        symlink_path.display()),
                    MB_ICONWARNING | MB_OKCANCEL
                ) == IDCANCEL {
                    return E_ABORT.ok()
                } else {
                    let symlink_str = symlink_path.to_str().unwrap();
                    if renamed_file_re.find(symlink_str).is_none() {
                        symlink_path = format!("{} (1)", symlink_path.to_str().unwrap()).into();
                    }
                }
            }

            while symlink_path.metadata().is_ok() { // exists, not following symlinks
                let symlink_str = symlink_path.to_str().unwrap();
                let mut err: Option<std::num::ParseIntError> = None;
                let renamed = renamed_file_re.replace(
                    symlink_str, |caps: &Captures| {
                        match str::parse::<>(&caps[1]) {
                            Err(e) => {
                                err = Some(e);
                                String::new()
                            },
                            std::result::Result::<usize, _>::Ok(n) => format!(" ({})", n+1)
                        }
                    });
                if let Some(e) = err {
                    crate::msgbox::msgBox(
                        "Drop for Symlink",
                        format!("Parse err:\n{}", e),
                        MB_ICONINFORMATION
                    );
                    return E_UNEXPECTED.ok()
                }
                symlink_path = renamed.to_string().into();
            }

            let Ok(metadata) = std::fs::metadata(&file_path) else { continue; };

            let result = 
                if metadata.is_dir() {
                    std::os::windows::fs::symlink_dir(file_path, symlink_path)
                } else {
                    std::os::windows::fs::symlink_file(file_path, symlink_path)
                };
            
            if let Err(e) = result {
                crate::msgbox::msgBox(
                    "Drop for Symlink",
                    format!("Error creating symlink:\n{}", e),
                    MB_ICONERROR
                );
            }
        }

        

        S_OK.ok()
    }

    fn GetCommandString(&self, _idCmd: usize, _uFlags: u32, _: *const u32, _pszName: PSTR, _cchName: u32) -> Result<()> {
        S_OK.ok()
    }
}