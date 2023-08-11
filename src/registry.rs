use windows::core::{Result, PCWSTR};
use windows::Win32::System::Registry::*;

use crate::winstr::WinStr;

pub enum RegValue {
    Sz(WinStr)
}

impl RegValue {
    fn get_data(&self) -> (REG_VALUE_TYPE, Option<&[u8]>) {
        match self {
            RegValue::Sz(s) => {
                (REG_SZ, Some(s.bytes_slice()))
            }
        }
    }
}

impl<S> From<S> for RegValue
where S: AsRef<str> {
    fn from(value: S) -> Self {
        Self::Sz(value.as_ref().into())
    }
}

#[derive(Debug)]
pub enum RegKeyMode {
    Create,
    Open
}

pub enum RegOp {
    Value(Option<WinStr>, RegValue),
    Key {
        mode: RegKeyMode,
        name: WinStr,
        operations: Vec<RegOp>
    }
}

pub struct RegBatch {
    pub key: HKEY,
    pub operations: Vec<RegOp>,
}

impl RegOp {
    fn apply(self, key: HKEY) -> Result<()> {
        match self {
            RegOp::Value(name, value) => {
                let (ty, bytes) = value.get_data();
                unsafe {
                    RegSetValueExW(key, WinStr::pwcstr_or_null(name.as_ref()), 0, ty, bytes)
                }
            }
            RegOp::Key { mode, name, operations } => {
                let mut result_hkey = HKEY::default();
                unsafe { match mode {
                    RegKeyMode::Create => RegCreateKeyExW(
                        key,
                        name.get_pcwstr(),
                        0, PCWSTR::null(),
                        REG_OPTION_NON_VOLATILE, KEY_WRITE, None,
                        &mut result_hkey,
                        None ),
                    RegKeyMode::Open => RegOpenKeyExW(
                        key,
                        name.get_pcwstr(),
                        0, KEY_ALL_ACCESS,
                        &mut result_hkey ),
                    }.ok()?;
                }
                
                RegBatch {
                    key: result_hkey,
                    operations
                }.apply()?;

                unsafe {
                    RegCloseKey(result_hkey)
                }
            }
        }.ok()
    }

    fn unapply(self, key: HKEY) {
        match self {
            RegOp::Value(name, ..) => unsafe {
                RegDeleteValueW(key, WinStr::pwcstr_or_null(name.as_ref()));
            },
            RegOp::Key { mode, name, mut operations } => {
                let mut result_hkey = HKEY::default();
                let open_result = unsafe {
                    RegOpenKeyExW(
                        key,
                        name.get_pcwstr(),
                        0, KEY_ALL_ACCESS,
                        &mut result_hkey )
                };

                
                if open_result.is_ok() {
                    operations.reverse();
                    RegBatch {
                        key: result_hkey,
                        operations
                    }.unapply();
    
                    unsafe {
                        RegCloseKey(result_hkey);
                        if let RegKeyMode::Create = mode {
                            RegDeleteKeyW(key, name.get_pcwstr());
                        }
                    }
                }
            }
        }
    }
}

impl RegBatch {
    pub fn apply(self) -> Result<()> {
        for op in self.operations {
            op.apply(self.key)?;
        }
        Ok(())
    }

    pub fn unapply(self) {
        for op in self.operations.into_iter().rev() {
            op.unapply(self.key);
        }
    }
}

#[macro_export]
macro_rules! vec_prefixed {
    ($first_item:expr, $rest_items:expr) => {
        {
            let mut items = vec![$first_item];
            items.extend($rest_items);
            items
        }
    };
}

#[macro_export]
macro_rules! reg_op {
    ($name:expr, $value:expr) => {
        registry::RegOp::CreateKey {
            name: format!($path),
            operations: vec![]
        }
    };
}

#[macro_export]
macro_rules! registry_items {
    () => {
        Vec::<registry::RegOp>::new()
    };
    // Open Key
    ([$path:literal] { $($ops:tt)* } $($rest:tt)*) => {
        $crate::vec_prefixed!{
            registry::RegOp::Key {
                mode: registry::RegKeyMode::Open,
                name: format!($path).into(),
                operations: $crate::registry_items!($($ops)*)
            },
            $crate::registry_items!($($rest)*)
        }
    };
    // Create Key
    (+ [$path:literal] { $($ops:tt)* } $($rest:tt)*) => {
        $crate::vec_prefixed!{
            registry::RegOp::Key {
                mode: registry::RegKeyMode::Create,
                name: format!($path).into(),
                operations: $crate::registry_items!($($ops)*)
            },
            $crate::registry_items!($($rest)*)
        }
    };
    // Set Default Value
    (=$val:tt $($rest:tt)*) => {
        $crate::vec_prefixed!{
            registry::RegOp::Value(None, $val.into()),
            $crate::registry_items!($($rest)*)
        }
    };
    // Set value named by variable, or parenthesized expression
    ($name:tt=$val:tt $($rest:tt)*) => {
        $crate::vec_prefixed!{
            registry::RegOp::Value(Some($name.into()), $val.into()),
            $crate::registry_items!($($rest)*)
        }
    };
}

#[macro_export]
macro_rules! registry {
    ($rootKey:ident, $($items:tt)+) => {
        registry::RegBatch {
            key: $rootKey,
            operations: $crate::registry_items!($($items)+)
        }
    };
}