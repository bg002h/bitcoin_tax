//! Best-effort mlock + zeroizing buffer.
//!
//! `SecretBuf` protects this buffer only — not SQLite's internal heap.

use zeroize::Zeroize;

pub struct SecretBuf {
    bytes: Vec<u8>,
    locked: bool,
}

impl SecretBuf {
    pub fn new(bytes: Vec<u8>) -> SecretBuf {
        let locked = Self::try_mlock(&bytes);
        if !locked {
            eprintln!("warning: mlock failed (RLIMIT_MEMLOCK?); decrypted vault may be swappable — use encrypted/disabled swap.");
        }
        SecretBuf { bytes, locked }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    #[cfg(unix)]
    fn try_mlock(b: &[u8]) -> bool {
        if b.is_empty() {
            return true;
        }
        unsafe { rustix::mm::mlock(b.as_ptr() as *mut _, b.len()).is_ok() }
    }

    // VirtualLock takes LPVOID (mut void*) but does not write through it; casting *const→*mut is safe. BOOL != 0 = success.
    #[cfg(windows)]
    fn try_mlock(b: &[u8]) -> bool {
        if b.is_empty() {
            return true;
        }
        unsafe {
            windows_sys::Win32::System::Memory::VirtualLock(b.as_ptr() as *mut _, b.len()) != 0
        }
    }

    #[cfg(not(any(unix, windows)))]
    fn try_mlock(_b: &[u8]) -> bool {
        false
    }
}

impl Drop for SecretBuf {
    fn drop(&mut self) {
        let len = self.bytes.len();
        self.bytes.zeroize();
        #[cfg(unix)]
        if self.locked && len > 0 {
            unsafe {
                let _ = rustix::mm::munlock(self.bytes.as_ptr() as *mut _, len);
            }
        }
        #[cfg(windows)]
        if self.locked && len > 0 {
            unsafe {
                let _ = windows_sys::Win32::System::Memory::VirtualUnlock(
                    self.bytes.as_ptr() as *mut _,
                    len,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_bytes_never_errors() {
        let s = SecretBuf::new(b"abc".to_vec());
        assert_eq!(s.as_slice(), b"abc");
        let _ = s.is_locked();
    }
}
