use crate::browser_process::life_span_handler::{WebviewPopupEvent, WebviewPopupSenderInner};
use bevy::prelude::Entity;
use cef::rc::{Rc, RcImpl};
use cef::{
    Browser, CefString, Frame, ImplRequestHandler, RequestHandler, WindowOpenDisposition,
    WrapRequestHandler, sys,
};
use std::os::raw::c_int;

pub struct RequestHandlerBuilder {
    object: *mut RcImpl<sys::_cef_request_handler_t, Self>,
    webview: Entity,
    tx: WebviewPopupSenderInner,
}

impl RequestHandlerBuilder {
    pub fn build(webview: Entity, tx: WebviewPopupSenderInner) -> RequestHandler {
        RequestHandler::new(Self {
            object: core::ptr::null_mut(),
            webview,
            tx,
        })
    }
}

impl Rc for RequestHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            core::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for RequestHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };
        Self {
            object,
            webview: self.webview,
            tx: self.tx.clone(),
        }
    }
}

impl WrapRequestHandler for RequestHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_request_handler_t, Self>) {
        self.object = object;
    }
}

impl ImplRequestHandler for RequestHandlerBuilder {
    fn on_open_urlfrom_tab(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        target_url: Option<&CefString>,
        _target_disposition: WindowOpenDisposition,
        _user_gesture: c_int,
    ) -> c_int {
        let url = target_url.map(|u| u.to_string()).unwrap_or_default();
        let _ = self.tx.send_blocking(WebviewPopupEvent {
            webview: self.webview,
            target_url: url,
        });
        1
    }

    fn get_raw(&self) -> *mut sys::_cef_request_handler_t {
        self.object.cast()
    }
}
