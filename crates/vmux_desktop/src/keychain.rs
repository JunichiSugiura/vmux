//! macOS keychain ACL fixup for CEF cookie encryption key.
//!
//! CEF/Chromium stores its cookie encryption key in
//! `login.keychain-db` under service `"Chromium Safe Storage"`,
//! account `"Chromium"`. The default ACL captured at item creation
//! time is bound to the **specific binary's cdhash** — meaning every
//! rebuild/update creates a new entry or triggers a keychain prompt,
//! and silently fails (logging the user out of Google etc.) if the
//! prompt is denied/missed.
//!
//! Chrome and Arc avoid this by registering a trusted app whose
//! Designated Requirement is `bundle id + team id` (not cdhash), so
//! any future binary signed with the same Developer ID identity
//! satisfies the requirement transparently.
//!
//! This module replicates that pattern. On startup (before CEF
//! initializes) we look up the safe-storage item, and if present
//! rewrite its access list so that `/Applications/Vmux.app` (or the
//! local-build equivalent) is the sole trusted app — captured by
//! its Designated Requirement, not its current cdhash.
//!
//! No-op on non-macOS, no-op if the item does not exist yet (CEF
//! creates it on first run with a bad ACL; we fix it on the next
//! launch).

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr;

const SERVICE: &[u8] = b"Chromium Safe Storage";
const ACCOUNT: &[u8] = b"Chromium";

/// Hardcoded trusted readers. The currently-running bundle is added on
/// top of these at runtime. Release builds always install at
/// `/Applications/Vmux.app`; local builds use a hash-suffixed name and
/// are picked up via the running-bundle path.
const FIXED_TRUSTED_APP_PATHS: &[&str] = &["/Applications/Vmux.app"];

// ----- FFI: Core Foundation -----
#[allow(non_camel_case_types)]
type CFTypeRef = *const c_void;
#[allow(non_camel_case_types)]
type CFAllocatorRef = *const c_void;
#[allow(non_camel_case_types)]
type CFStringRef = *const c_void;
#[allow(non_camel_case_types)]
type CFArrayRef = *const c_void;
#[allow(non_camel_case_types)]
type CFIndex = isize;

#[repr(C)]
struct CFArrayCallBacks {
    version: CFIndex,
    retain: *const c_void,
    release: *const c_void,
    copy_description: *const c_void,
    equal: *const c_void,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFAllocatorDefault: CFAllocatorRef;
    static kCFTypeArrayCallBacks: CFArrayCallBacks;

    fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: u32,
        is_external: u8,
    ) -> CFStringRef;

    fn CFArrayCreate(
        alloc: CFAllocatorRef,
        values: *const CFTypeRef,
        num_values: CFIndex,
        callbacks: *const CFArrayCallBacks,
    ) -> CFArrayRef;

    fn CFRelease(cf: CFTypeRef);
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

// ----- FFI: Security framework -----
#[allow(non_camel_case_types)]
type SecKeychainItemRef = *mut c_void;
#[allow(non_camel_case_types)]
type SecAccessRef = *mut c_void;
#[allow(non_camel_case_types)]
type SecTrustedApplicationRef = *mut c_void;
#[allow(non_camel_case_types)]
type OSStatus = i32;

const ERR_SEC_SUCCESS: OSStatus = 0;
const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecKeychainFindGenericPassword(
        keychain_or_array: CFTypeRef,
        service_name_length: u32,
        service_name: *const u8,
        account_name_length: u32,
        account_name: *const u8,
        password_length: *mut u32,
        password_data: *mut *mut c_void,
        item_ref: *mut SecKeychainItemRef,
    ) -> OSStatus;

    fn SecKeychainItemSetAccess(item_ref: SecKeychainItemRef, access: SecAccessRef) -> OSStatus;

    fn SecTrustedApplicationCreateFromPath(
        path: *const i8,
        app: *mut SecTrustedApplicationRef,
    ) -> OSStatus;

    fn SecAccessCreate(
        descriptor: CFStringRef,
        trusted_list: CFArrayRef,
        access: *mut SecAccessRef,
    ) -> OSStatus;
}

/// Ensure the `Chromium Safe Storage` keychain item (if it exists) has an
/// ACL whose trusted-app entries reference `/Applications/Vmux.app` (and
/// the local-build path) by their Designated Requirement, so future signed
/// builds with the same Developer ID identity inherit access without
/// prompting the user.
pub fn ensure_chromium_safe_storage_acl() {
    // Only manage the ACL when running from an installed `.app` bundle.
    // In dev (`make run-mac`) the binary lives under `target/`, so the
    // trusted-app list would only contain `/Applications/Vmux.app` —
    // locking the dev binary out and triggering a prompt for every CEF
    // helper subprocess on every launch. Letting CEF use its default
    // cdhash-based ACL means a one-time "Always Allow" per helper that
    // sticks until the dev binary is rebuilt.
    if running_app_bundle().is_none() {
        eprintln!("vmux: keychain ACL fixup skipped (dev binary, no .app bundle)");
        return;
    }
    if let Err(e) = ensure_inner() {
        eprintln!("vmux: keychain ACL fixup skipped: {e}");
    }
}

/// Walk up from the current executable to find an enclosing `.app`
/// bundle. Returns `None` for bare-binary runs (e.g. `cargo run`).
fn running_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|s| s.to_str()) == Some("app") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn ensure_inner() -> Result<(), String> {
    // Look up the item. If absent, do nothing — CEF will create it on
    // first run; we fix the ACL on the next launch.
    let item = unsafe {
        let mut item: SecKeychainItemRef = ptr::null_mut();
        let status = SecKeychainFindGenericPassword(
            ptr::null(),
            SERVICE.len() as u32,
            SERVICE.as_ptr(),
            ACCOUNT.len() as u32,
            ACCOUNT.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut item,
        );
        match status {
            ERR_SEC_SUCCESS => item,
            ERR_SEC_ITEM_NOT_FOUND => return Ok(()),
            s => return Err(format!("SecKeychainFindGenericPassword failed: {s}")),
        }
    };

    // Collect candidate trusted-app paths: the fixed list plus the
    // currently-running bundle (if it lives inside an .app). De-dup
    // and skip non-existent paths.
    let mut paths: Vec<PathBuf> = FIXED_TRUSTED_APP_PATHS.iter().map(PathBuf::from).collect();
    if let Some(running) = running_app_bundle()
        && !paths.contains(&running)
    {
        paths.push(running);
    }

    let mut trusted: Vec<SecTrustedApplicationRef> = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let cstr = match path.to_str().and_then(|s| std::ffi::CString::new(s).ok()) {
            Some(c) => c,
            None => continue,
        };
        unsafe {
            let mut app: SecTrustedApplicationRef = ptr::null_mut();
            let status = SecTrustedApplicationCreateFromPath(cstr.as_ptr(), &mut app);
            if status == ERR_SEC_SUCCESS && !app.is_null() {
                trusted.push(app);
            } else {
                eprintln!(
                    "vmux: SecTrustedApplicationCreateFromPath({}) failed: {status}",
                    path.display()
                );
            }
        }
    }

    if trusted.is_empty() {
        unsafe { CFRelease(item as CFTypeRef) };
        return Err("no trusted app paths exist on disk".into());
    }

    unsafe {
        let trusted_array = CFArrayCreate(
            kCFAllocatorDefault,
            trusted.as_ptr() as *const CFTypeRef,
            trusted.len() as CFIndex,
            &kCFTypeArrayCallBacks,
        );
        if trusted_array.is_null() {
            for app in &trusted {
                CFRelease(*app as CFTypeRef);
            }
            CFRelease(item as CFTypeRef);
            return Err("CFArrayCreate returned null".into());
        }

        let descriptor = CFStringCreateWithBytes(
            kCFAllocatorDefault,
            SERVICE.as_ptr(),
            SERVICE.len() as CFIndex,
            K_CF_STRING_ENCODING_UTF8,
            0,
        );

        let mut access: SecAccessRef = ptr::null_mut();
        let status = SecAccessCreate(descriptor, trusted_array, &mut access);
        let create_result = if status == ERR_SEC_SUCCESS && !access.is_null() {
            let s = SecKeychainItemSetAccess(item, access);
            CFRelease(access as CFTypeRef);
            if s == ERR_SEC_SUCCESS {
                Ok(())
            } else {
                Err(format!("SecKeychainItemSetAccess failed: {s}"))
            }
        } else {
            Err(format!("SecAccessCreate failed: {status}"))
        };

        CFRelease(descriptor as CFTypeRef);
        CFRelease(trusted_array as CFTypeRef);
        for app in &trusted {
            CFRelease(*app as CFTypeRef);
        }
        CFRelease(item as CFTypeRef);

        if create_result.is_ok() {
            eprintln!(
                "vmux: keychain ACL for 'Chromium Safe Storage' updated ({} trusted app(s))",
                trusted.len()
            );
        }
        create_result
    }
}
