use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn ancho(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

pub fn abrir(objetivo: &OsStr) -> bool {
    let destino = ancho(objetivo);
    if destino.len() < 2 || destino[..destino.len() - 1].iter().any(|&c| c < 0x20) {
        return false;
    }
    let verbo = ancho(OsStr::new("open"));
    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let r = ShellExecuteW(std::ptr::null_mut(), verbo.as_ptr(), destino.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        r as usize > 32
    }
}

pub fn copiar(texto: &str) -> bool {
    let datos: Vec<u16> = texto.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        let mut hecho = false;
        if EmptyClipboard() != 0 {
            let h = GlobalAlloc(GMEM_MOVEABLE, datos.len() * 2);
            if !h.is_null() {
                let p = GlobalLock(h) as *mut u16;
                if p.is_null() {
                    GlobalFree(h);
                } else {
                    std::ptr::copy_nonoverlapping(datos.as_ptr(), p, datos.len());
                    GlobalUnlock(h);
                    if SetClipboardData(u32::from(CF_UNICODETEXT), h).is_null() {
                        GlobalFree(h);
                    } else {
                        hecho = true;
                    }
                }
            }
        }
        CloseClipboard();
        hecho
    }
}

pub fn pegar() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let mut texto = None;
        let h = GetClipboardData(u32::from(CF_UNICODETEXT));
        if !h.is_null() {
            let maximo = GlobalSize(h) / 2;
            let p = GlobalLock(h) as *const u16;
            if !p.is_null() {
                let datos = std::slice::from_raw_parts(p, maximo);
                let fin = datos.iter().position(|&c| c == 0).unwrap_or(maximo);
                texto = Some(String::from_utf16_lossy(&datos[..fin]));
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        texto
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn abrir_rechaza_caracteres_de_control() {
        assert!(!abrir(OsStr::new("https://example.com/\nmal")));
        assert!(!abrir(OsStr::new("")));
    }
}
