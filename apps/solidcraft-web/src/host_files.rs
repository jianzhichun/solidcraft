//! Files of the page that embeds this build (a desktop host running the native MCP server beside it, for one). The engine's
//! files live in memory here (`vfs`); a command told to write to a path of the host's computer (`ui.screenshot` into the MCP
//! server's scratch folder, `file.export` into the host's work folder) writes it to memory, where only the host can take it.
//! Once the page calls `solidcraft.takeHostWrites()` such writes are kept for it (and only then, so a plain web visit keeps
//! nothing extra); `solidcraft.putFile` hands the engine a host file a command is about to read. Which host paths a request
//! may touch is the host's to check (the request's `allow_paths`): this module only carries bytes.
//!
//! The path logic is plain Rust and tested natively; the browser calls are wasm-only.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

/// Most a page keeps for its host between two takes: older writes are dropped first.
const MAX_KEPT_BYTES: usize = 256 << 20;
const MAX_KEPT_FILES: usize = 64;
/// Largest file a host may hand the engine.
pub const MAX_PUT_BYTES: usize = 256 << 20;

thread_local! {
    static HOSTED: Cell<bool> = const { Cell::new(false) };
    static KEPT: RefCell<VecDeque<(String, Vec<u8>)>> = const { RefCell::new(VecDeque::new()) };
}

/// A path of the host's computer: absolute (`/…` or `C:\…`), and not one of the places this build keeps for itself.
pub fn is_host_path(path: &str) -> bool {
    let browser = [crate::disk::OPFS_PREFIX, crate::disk::PREFIX, crate::disk::DOWNLOADS_PREFIX, crate::disk::UPLOAD_PREFIX];
    let temp = solidcraft_engine::io::vfs::BROWSER_TEMP;
    let drive = path.as_bytes().get(1) == Some(&b':') && path.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    (path.starts_with('/') || drive)
        && !browser.iter().any(|p| path.starts_with(p))
        && path != temp
        && !path.starts_with(&format!("{temp}/"))
        && !path.split(['/', '\\']).any(|part| part == "..")
}

/// A write `flush` did not place: kept for the host when there is one. True when kept.
pub fn keep(path: &str, bytes: Vec<u8>) -> bool {
    if !HOSTED.with(Cell::get) || !is_host_path(path) {
        return false;
    }
    KEPT.with(|k| {
        let mut k = k.borrow_mut();
        k.retain(|(p, _)| p != path);
        k.push_back((path.to_string(), bytes));
        while k.len() > MAX_KEPT_FILES || k.iter().map(|(_, b)| b.len()).sum::<usize>() > MAX_KEPT_BYTES {
            k.pop_front();
        }
    });
    true
}

/// Everything kept since the last take; from the first take on, writes are kept.
pub fn take() -> Vec<(String, Vec<u8>)> {
    HOSTED.with(|h| h.set(true));
    KEPT.with(|k| k.borrow_mut().drain(..).collect())
}

/// Whether a host file may be handed to the engine at `path`.
pub fn put_refusal(path: &str, len: usize) -> Option<String> {
    if !is_host_path(path) {
        return Some(format!("{path}: not a path of the host's computer"));
    }
    (len > MAX_PUT_BYTES).then(|| format!("{path}: larger than {} MiB", MAX_PUT_BYTES >> 20))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_paths_are_absolute_and_outside_the_builds_own_places() {
        for p in ["/var/folders/1q/T/solidcraft-mcp-1/shot-0.png", "/Users/a/Designs/plate.step", "C:\\Users\\a\\plate.step"] {
            assert!(is_host_path(p), "{p}");
        }
        for p in [
            "/opfs/Plate.solidcraft",
            "/disk/1-2/Plate.solidcraft",
            "/downloads/x.step",
            "/upload/x.step",
            "/tmp/solidcraft-shot.png",
            "plate.step",
            "/a/../etc/passwd",
        ] {
            assert!(!is_host_path(p), "{p}");
        }
    }

    #[test]
    fn writes_are_kept_only_once_a_host_takes_them_and_the_newest_wins() {
        assert!(!keep("/h/a.png", vec![1]), "no host yet");
        assert!(take().is_empty());
        assert!(keep("/h/a.png", vec![1]) && keep("/h/a.png", vec![2]) && !keep("/opfs/x", vec![3]));
        assert_eq!(take(), vec![("/h/a.png".to_string(), vec![2])]);
        assert!(take().is_empty());
        assert!(put_refusal("/upload/x.step", 1).is_some());
        assert!(put_refusal("/h/x.step", MAX_PUT_BYTES + 1).is_some() && put_refusal("/h/x.step", 10).is_none());
    }
}
