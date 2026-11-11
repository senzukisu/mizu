//! OLE drop target for windows that host views visually: files or text dragged from
//! Explorer or other apps are forwarded to the view under the pointer.

use std::cell::RefCell;

use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2CompositionController3;
use windows::Win32::Foundation::{HWND, POINT, POINTL};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Ole::{DROPEFFECT, DROPEFFECT_NONE, IDropTarget, IDropTarget_Impl, RegisterDragDrop, RevokeDragDrop};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows_core::{Ref, implement};

#[implement(IDropTarget)]
struct DropTarget {
    hwnd: HWND,
    state: RefCell<Option<(ICoreWebView2CompositionController3, Option<IDataObject>)>>,
}

impl DropTarget {
    /// Moves the drag to whatever view is under `pt`, sending leave/enter as needed.
    fn retarget(&self, data: Option<&IDataObject>, keys: u32, pt: &POINTL, effect: *mut DROPEFFECT) -> windows_core::Result<()> {
        let screen = POINT { x: pt.x, y: pt.y };
        let target = crate::host::drag_target_at(self.hwnd, screen);
        let mut state = self.state.borrow_mut();
        let data = data.cloned().or_else(|| state.as_ref().and_then(|(_, d)| d.clone()));
        match target {
            Some((comp, local)) => {
                let same = state.as_ref().is_some_and(|(c, _)| c == &comp);
                // SAFETY: COM calls on the UI thread; `effect` is the caller's out pointer.
                unsafe {
                    if same {
                        comp.DragOver(keys, local, effect.cast())?;
                    } else {
                        if let Some((old, _)) = state.take() {
                            let _ = old.DragLeave();
                        }
                        if let Some(d) = &data {
                            comp.DragEnter(d, keys, local, effect.cast())?;
                        }
                    }
                }
                *state = Some((comp, data));
            }
            None => {
                if let Some((old, _)) = state.take() {
                    // SAFETY: COM call on the UI thread.
                    unsafe {
                        let _ = old.DragLeave();
                    }
                }
                if !effect.is_null() {
                    // SAFETY: caller-provided out pointer.
                    unsafe { *effect = DROPEFFECT_NONE };
                }
            }
        }
        Ok(())
    }
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(&self, data: Ref<'_, IDataObject>, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows_core::Result<()> {
        self.state.borrow_mut().take();
        self.retarget(data.as_ref(), keys.0, pt, effect)
    }

    fn DragOver(&self, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows_core::Result<()> {
        self.retarget(None, keys.0, pt, effect)
    }

    fn DragLeave(&self) -> windows_core::Result<()> {
        if let Some((old, _)) = self.state.borrow_mut().take() {
            // SAFETY: COM call on the UI thread.
            unsafe {
                let _ = old.DragLeave();
            }
        }
        Ok(())
    }

    fn Drop(&self, data: Ref<'_, IDataObject>, keys: MODIFIERKEYS_FLAGS, pt: &POINTL, effect: *mut DROPEFFECT) -> windows_core::Result<()> {
        let screen = POINT { x: pt.x, y: pt.y };
        let target = crate::host::drag_target_at(self.hwnd, screen);
        let state = self.state.borrow_mut().take();
        match (target, data.as_ref()) {
            (Some((comp, local)), Some(d)) => {
                // SAFETY: COM call on the UI thread.
                unsafe {
                    if state.as_ref().is_none_or(|(c, _)| c != &comp) {
                        if let Some((old, _)) = &state {
                            let _ = old.DragLeave();
                        }
                        let mut e = 0u32;
                        let _ = comp.DragEnter(d, keys.0, local, &mut e);
                    }
                    comp.Drop(d, keys.0, local, effect.cast())?;
                }
            }
            _ => {
                if let Some((old, _)) = state {
                    // SAFETY: COM call on the UI thread.
                    unsafe {
                        let _ = old.DragLeave();
                    }
                }
                if !effect.is_null() {
                    // SAFETY: caller-provided out pointer.
                    unsafe { *effect = DROPEFFECT_NONE };
                }
            }
        }
        let _ = crate::host::current_drag_target(self.hwnd);
        Ok(())
    }
}

pub(crate) fn register(hwnd: HWND) {
    let target: IDropTarget = DropTarget { hwnd, state: RefCell::new(None) }.into();
    // SAFETY: OLE is initialised on the UI thread (OleInitialize in main); the system keeps a
    // reference to the target until RevokeDragDrop.
    unsafe {
        let _ = RegisterDragDrop(hwnd, &target);
    }
}

pub(crate) fn revoke(hwnd: HWND) {
    // SAFETY: revoking our own registration.
    unsafe {
        let _ = RevokeDragDrop(hwnd);
    }
}
