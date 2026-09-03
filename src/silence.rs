use std::ptr;

/// Clears an arbitrary audio buffer in-place using raw FFI pointers.
///
/// # Real-Time Performance & Thread Safety Invariants
///
/// 1. **Zero Heap Allocations:** Must never invoke vector allocations, string formatting,
///    or dynamic memory dispatch. Allocating on the CoreAudio render thread triggers system
///    allocator locks, leading to priority inversion and audio dropouts (glitches).
/// 2. **Zero Bounds-Checking Overhead:** Operates directly on raw pointers (`*mut u8`) using
///    `ptr::write_bytes` (`llvm.memset`). This avoids constructing intermediate Rust slices
///    (`std::slice::from_raw_parts_mut`) inside the high-frequency interrupt loop, which adds
///    slice-length calculation overhead and potential panic branches.
/// 3. **SIMD Auto-Vectorization:** LLVM lowers `ptr::write_bytes` directly to hardware vector
///    instructions (e.g., ARM64 NEON register stores on Apple Silicon), clearing memory blocks
///    in optimal CPU cycles.
///
/// # Safety
///
/// - `buffer_ptr` must be non-null and point to a valid, writable memory block of at least `byte_size` bytes.
/// - The caller must guarantee that no other thread holds a reference to this memory block for the
///   duration of the call (CoreAudio hardware buffer ownership model handles this).
#[inline(always)]
pub unsafe fn clear_buffer(buffer_ptr: *mut u8, byte_size: usize) {
    // Early return guards to prevent invalid memory writes on corrupted HAL structs
    if buffer_ptr.is_null() || byte_size == 0 {
        return;
    }

    // Direct C-style memset over raw memory address space
    let fill_byte_value: u8 = 0;
    ptr::write_bytes(buffer_ptr, fill_byte_value, byte_size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_buffer_zeroes_all_bytes() {
        let mut buffer = vec![0xFFu8; 1024];
        let len = buffer.len();
        unsafe {
            clear_buffer(buffer.as_mut_ptr(), len);
        }
        assert!(buffer.iter().all(|&byte| byte == 0));
    }

    #[test]
    fn test_clear_buffer_empty_slice() {
        let mut buffer: [u8; 0] = [];
        unsafe {
            clear_buffer(buffer.as_mut_ptr(), 0);
        }
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_silence_float_representation() {
        let mut buffer = vec![0xFFu8; 8];
        let len = buffer.len();
        unsafe {
            clear_buffer(buffer.as_mut_ptr(), len);
        }

        let floats: &[f32] = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const f32,
                buffer.len() / std::mem::size_of::<f32>(),
            )
        };

        for &sample in floats {
            assert_eq!(sample, 0.0f32);
        }
    }
}
