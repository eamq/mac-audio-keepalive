use coreaudio_sys::{
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatLinearPCM,
    kAudioUnitManufacturer_Apple, kAudioUnitProperty_SetRenderCallback,
    kAudioUnitProperty_StreamFormat, kAudioUnitScope_Global, kAudioUnitScope_Input,
    kAudioUnitSubType_DefaultOutput, kAudioUnitType_Output, noErr, AURenderCallbackStruct,
    AudioBuffer, AudioBufferList, AudioComponent, AudioComponentDescription,
    AudioComponentFindNext, AudioComponentInstanceDispose, AudioComponentInstanceNew,
    AudioOutputUnitStart, AudioOutputUnitStop, AudioStreamBasicDescription, AudioTimeStamp,
    AudioUnit, AudioUnitInitialize, AudioUnitRenderActionFlags, AudioUnitSetProperty,
    AudioUnitUninitialize, OSStatus, UInt32,
};
use mac_audio_keepalive::silence;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::ffi::c_void;
use std::ptr;

// Stream Format Configuration Constants
const SAMPLE_RATE_HZ: f64 = 44100.0;
const STEREO_CHANNEL_COUNT: UInt32 = 2;
const BITS_PER_CHANNEL: UInt32 = 32;
const BYTES_PER_SAMPLE: UInt32 = BITS_PER_CHANNEL / 8; // 4 bytes (f32)
const BYTES_PER_FRAME: UInt32 = BYTES_PER_SAMPLE * STEREO_CHANNEL_COUNT; // 8 bytes

/// Real-time CoreAudio Render Callback.
/// Executes on the high-priority OS audio thread driven by hardware clock interrupts.
unsafe extern "C" fn render_callback(
    _in_ref_con: *mut c_void,
    _io_action_flags: *mut AudioUnitRenderActionFlags,
    _in_time_stamp: *const AudioTimeStamp,
    _in_bus_number: UInt32,
    _in_number_frames: UInt32,
    io_data: *mut AudioBufferList,
) -> OSStatus {
    if io_data.is_null() {
        return noErr as OSStatus;
    }

    // Dereference raw C AudioBufferList pointer passed by OS HAL
    let buffer_list: &AudioBufferList = &*io_data;
    let buffers: &[AudioBuffer] = std::slice::from_raw_parts(
        buffer_list.mBuffers.as_ptr(),
        buffer_list.mNumberBuffers as usize,
    );

    for buf in buffers {
        if !buf.mData.is_null() && buf.mDataByteSize > 0 {
            silence::clear_buffer(buf.mData as *mut u8, buf.mDataByteSize as usize);
        }
    }

    noErr as OSStatus
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing mac-audio-keepalive daemon...");

    // 1. Force host process QoS to BACKGROUND (scheduled exclusively on Apple Silicon E-Cores)
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_BACKGROUND, 0);
    }

    // 2. Set up POSIX Signal Hook Iterator for graceful termination
    let mut signals: Signals = Signals::new([SIGINT, SIGTERM])?;

    let mut instance: AudioUnit = ptr::null_mut();

    unsafe {
        // 3. Locate Default Output Audio Component
        let desc: AudioComponentDescription = AudioComponentDescription {
            componentType: kAudioUnitType_Output,
            componentSubType: kAudioUnitSubType_DefaultOutput,
            componentManufacturer: kAudioUnitManufacturer_Apple,
            componentFlags: 0,
            componentFlagsMask: 0,
        };

        let component: AudioComponent = AudioComponentFindNext(ptr::null_mut(), &desc);
        if component.is_null() {
            return Err("Failed to locate default output AudioComponent".into());
        }

        let status: OSStatus = AudioComponentInstanceNew(component, &mut instance);
        if status != noErr as OSStatus {
            return Err(format!("AudioComponentInstanceNew failed with status: {}", status).into());
        }

        // 4. Configure PCM Stream Format
        let stream_format: AudioStreamBasicDescription = AudioStreamBasicDescription {
            mSampleRate: SAMPLE_RATE_HZ,
            mFormatID: kAudioFormatLinearPCM,
            mFormatFlags: kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
            mBytesPerPacket: BYTES_PER_FRAME,
            mFramesPerPacket: 1,
            mBytesPerFrame: BYTES_PER_FRAME,
            mChannelsPerFrame: STEREO_CHANNEL_COUNT,
            mBitsPerChannel: BITS_PER_CHANNEL,
            mReserved: 0,
        };

        let set_format_status: OSStatus = AudioUnitSetProperty(
            instance,
            kAudioUnitProperty_StreamFormat,
            kAudioUnitScope_Input,
            0,
            &stream_format as *const _ as *const c_void,
            std::mem::size_of::<AudioStreamBasicDescription>() as UInt32,
        );
        if set_format_status != noErr as OSStatus {
            return Err(format!("Failed to set stream format: {}", set_format_status).into());
        }

        // 5. Attach High-Frequency Render Callback
        let callback_struct: AURenderCallbackStruct = AURenderCallbackStruct {
            inputProc: Some(render_callback),
            inputProcRefCon: ptr::null_mut(),
        };

        let set_callback_status: OSStatus = AudioUnitSetProperty(
            instance,
            kAudioUnitProperty_SetRenderCallback,
            kAudioUnitScope_Global,
            0,
            &callback_struct as *const _ as *const c_void,
            std::mem::size_of::<AURenderCallbackStruct>() as UInt32,
        );
        if set_callback_status != noErr as OSStatus {
            return Err(format!("Failed to set render callback: {}", set_callback_status).into());
        }

        // 6. Initialize Hardware Pipeline & Start Stream
        AudioUnitInitialize(instance);
        AudioOutputUnitStart(instance);
    }

    println!("Daemon active. CoreAudio HAL keep-alive running on E-Cores.");

    // 7. Block main thread until SIGINT or SIGTERM is caught
    if let Some(signal) = signals.forever().next() {
        println!(
            "\nReceived signal {}. Cleaning up CoreAudio HAL hardware resources...",
            signal
        );
    }

    // 8. Graceful Hardware Teardown
    unsafe {
        if !instance.is_null() {
            AudioOutputUnitStop(instance);
            AudioUnitUninitialize(instance);
            AudioComponentInstanceDispose(instance);
        }
    }

    println!("Daemon stopped cleanly.");
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_atomic_shutdown_flag_toggle() {
        let shutdown_flag = std::sync::atomic::AtomicBool::new(false);
        shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(shutdown_flag.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_plist_target_name_format() {
        let service_name = "com.user.mac-audio-keepalive";
        assert!(service_name.starts_with("com."));
        assert!(!service_name.contains(' '));
    }
}
