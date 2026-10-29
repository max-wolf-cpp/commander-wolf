use std::fs::Metadata;
use std::path::Path;

use crate::file_operations::progress::OperationObserver;

pub(super) fn copy(
    source: &Path,
    destination: &Path,
    metadata: &Metadata,
    observer: &mut dyn OperationObserver,
) -> Result<(), String> {
    platform::copy(source, destination, metadata, observer)?;
    observer.check_cancelled()?;
    observer.finish_file();

    Ok(())
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};

    pub(super) fn copy(
        source: &Path,
        destination: &Path,
        metadata: &Metadata,
        observer: &mut dyn OperationObserver,
    ) -> Result<(), String> {
        let mut input = File::open(source).map_err(|error| error.to_string())?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            // Private source contents must not become public while staging is written.
            options.mode(metadata.permissions().mode());
        }
        let mut output = options
            .open(destination)
            .map_err(|error| error.to_string())?;

        let mut buffer = vec![0; 256 * 1024];
        loop {
            observer.check_cancelled()?;
            let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }

            #[cfg(test)]
            super::super::faults::check("read", source).map_err(|error| error.to_string())?;
            output
                .write_all(&buffer[..count])
                .map_err(|error| error.to_string())?;
            observer.advance(count as u64);
            #[cfg(test)]
            super::super::faults::check("write", source).map_err(|error| error.to_string())?;
        }

        output
            .set_permissions(metadata.permissions())
            .map_err(|error| error.to_string())
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[repr(C)]
    union LargeInteger {
        quad_part: i64,
    }

    type ProgressRoutine = unsafe extern "system" fn(
        LargeInteger,
        LargeInteger,
        LargeInteger,
        LargeInteger,
        u32,
        u32,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) -> u32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CopyFileExW(
            source: *const u16,
            destination: *const u16,
            progress: Option<ProgressRoutine>,
            data: *mut c_void,
            cancel: *mut i32,
            flags: u32,
        ) -> i32;
    }

    struct Context<'a> {
        observer: &'a mut dyn OperationObserver,
        source: &'a Path,
        transferred: u64,
        total: u64,
        error: Option<String>,
    }

    pub(super) fn copy(
        source: &Path,
        destination: &Path,
        metadata: &Metadata,
        observer: &mut dyn OperationObserver,
    ) -> Result<(), String> {
        // Canonical parents retain long-path support for the not-yet-created staging file.
        let source_path = source.canonicalize().map_err(|error| error.to_string())?;
        let destination_path = destination
            .parent()
            .unwrap_or(Path::new("."))
            .canonicalize()
            .map_err(|error| error.to_string())?
            .join(
                destination
                    .file_name()
                    .ok_or("Destination has no file name")?,
            );
        let source_wide = wide_path(&source_path)?;
        let destination_wide = wide_path(&destination_path)?;

        let mut context = Context {
            observer,
            source,
            transferred: 0,
            total: metadata.len().max(1),
            error: None,
        };
        context.observer.check_cancelled()?;

        // Keep Windows attributes and alternate streams, as the former fs::copy did.
        let copied = unsafe {
            CopyFileExW(
                source_wide.as_ptr(),
                destination_wide.as_ptr(),
                Some(progress),
                (&mut context as *mut Context<'_>).cast(),
                std::ptr::null_mut(),
                1, // COPY_FILE_FAIL_IF_EXISTS
            )
        };
        if copied == 0 {
            let error = std::io::Error::last_os_error();
            return Err(context.error.unwrap_or_else(|| error.to_string()));
        }

        Ok(())
    }

    impl Context<'_> {
        fn report_progress(
            &mut self,
            total: LargeInteger,
            transferred: LargeInteger,
        ) -> Result<(), String> {
            self.observer.check_cancelled()?;

            let total = unsafe { total.quad_part }.max(0) as u64;
            let transferred = unsafe { transferred.quad_part }.max(0) as u64;
            if self.transferred == 0 && total.max(1) != self.total {
                self.total = total.max(1);
                self.observer.begin_file(self.source, self.total);
            }

            let delta = transferred.saturating_sub(self.transferred);
            if delta > 0 {
                self.observer.advance(delta);
                #[cfg(test)]
                for step in ["read", "write"] {
                    super::super::faults::check(step, self.source)
                        .map_err(|error| error.to_string())?;
                }
            }
            self.transferred = transferred;

            self.observer.check_cancelled()
        }
    }

    unsafe extern "system" fn progress(
        total: LargeInteger,
        transferred: LargeInteger,
        _stream_size: LargeInteger,
        _stream_transferred: LargeInteger,
        _stream_number: u32,
        _reason: u32,
        _source: *mut c_void,
        _destination: *mut c_void,
        data: *mut c_void,
    ) -> u32 {
        // CopyFileExW invokes callbacks synchronously, within the context's lifetime.
        let context = unsafe { &mut *data.cast::<Context<'_>>() };
        let result = catch_unwind(AssertUnwindSafe(|| {
            context.report_progress(total, transferred)
        }));

        match result {
            Ok(Ok(())) => 0, // PROGRESS_CONTINUE
            result => {
                context.error = Some(match result {
                    Ok(Err(error)) => error,
                    _ => "Copy progress callback failed".into(),
                });
                1 // PROGRESS_CANCEL also removes the incomplete native copy.
            }
        }
    }

    fn wide_path(path: &Path) -> Result<Vec<u16>, String> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err("File path contains a null character".into());
        }

        wide.push(0);

        Ok(wide)
    }
}
