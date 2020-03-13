/*
 * Rust "port" of the `man fanotify` example program offered as part of release 4.04 of the Linux
 * man-pages project.
 */

use nix::sys::fanotify::*;
use semver_parser::version;
use std::os::unix::io::AsRawFd;
use std::{env, fs};

fn handle_events(fd: Fanotify) -> Result<(), Box<dyn std::error::Error>> {
    /* Loop while events can be read from fanotify file descriptor */
    loop {
        /* Read some events */
        /* e.fd contains either None, indicating a queue overflow, or a file descriptor (a
         * nonnegative integer). Here we simply ignore queue overflow (by filtering out cases where
         * `e.fd` is None). */
        for e in fd.read_events()?.iter().filter(|e| e.file.is_some()) {
            // NOTE(@cpu): safe to unconditionally unwrap here because of `filter` above.
            let f = e.file.as_ref().unwrap();

            if e.mask.contains(MaskFlags::FAN_OPEN_PERM) {
                print!("FAN_OPEN_PERM: ");

                /* Allow file to be opened */
                fd.respond(FanotifyResponse {
                    fd: f.as_raw_fd(),
                    response: FanotifyPermissionResponse::FAN_ALLOW,
                })?;
            }

            if e.mask.contains(MaskFlags::FAN_CLOSE_WRITE) {
                print!("FAN_CLOSE_WRITE: ");
            }

            /* Retrieve and print pathname of the accessed file */
            let procfd_path = fs::read_link(format!("/proc/self/fd/{}", f.as_raw_fd()))?;
            print!("File {} ", procfd_path.display());

            /* Retrieve and print exe of the accessing pid (if possible) */
            match fs::read_link(format!("/proc/{}/exe", e.pid)) {
                Ok(ref exe) => {
                    print!("Exe {}", exe.display());
                }
                _ => {
                    print!("Pid {} (gone)", e.pid);
                }
            };
            println!();

            /* Advance to the next event */
        }
    }
}

// can_mark_full_filesystem returns Some with a kernel version if the Kernel is >= 4.20.0 and can
// use FAN_MARK_FILESYSTEM with `fanotify_mark`, or None if the Kernel is too old.
fn can_mark_full_filesystem() -> Option<version::Version> {
    // Using FAN_MARK_FILESYSTEM requires a Linux kernel version >= 4.20.0
    // NOTE: unwrap() is safe here. The parsed version is a well formed constant.
    let mark_filesystem_requires = version::parse("4.20.0").unwrap();

    // Find the current kernel version using the uname release field. This may come with extra junk
    // at the end after a `-`, e.g. `4.4.0-174-generic`, so split the release field by '-' and
    // collect the parts into a vec.
    let uname = nix::sys::utsname::uname();
    let release_parts: Vec<&str> = uname.release().split('-').collect();

    // Parse up to the first '-', if the resulting version is >= mark_filesystem_requires per
    // semver then return true, otherwise if anything fails to parse or the version isn't >=
    // mark_filesystem_requires, return false
    release_parts
        .first()
        .and_then(|x| version::parse(x).ok())
        .and_then(|x| {
            if x >= mark_filesystem_requires {
                Some(x)
            } else {
                None
            }
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    // skip program name
    args.next();

    /* Check mount point is supplied */
    let input_path = match args.next() {
        None => "./".to_string(),
        Some(p) => p,
    };

    /* Create the file descriptor for accessing the fanotify API */
    let fd = Fanotify::init(
        // TODO(@cpu): should use FAN_NONBLOCK like the original example. See
        // https://github.com/cpu/rfanotify/issues/2
        InitFlags::FAN_CLOEXEC | InitFlags::FAN_CLASS_CONTENT,
        EventFlags::O_RDONLY | EventFlags::O_LARGEFILE,
    )?;

    // If the current kernel version is new enough, use FAN_MARK_FILESYSTEM instead of
    // FAN_MARK_MOUNT.
    let mark_flags = match can_mark_full_filesystem() {
        Some(_) => MarkFlags::FAN_MARK_ADD | MarkFlags::FAN_MARK_FILESYSTEM,
        None => MarkFlags::FAN_MARK_ADD | MarkFlags::FAN_MARK_MOUNT,
    };

    /* Mark the mount|filesystem for:
     * - permission events before opening files
     * - notification events after closing a write-enabled file descriptor
     */
    fd.mark(
        mark_flags,
        MaskFlags::FAN_OPEN_PERM | MaskFlags::FAN_CLOSE_WRITE,
        AT_FDCWD,
        input_path.as_str(),
    )?;

    /* Fanotify events may be available */
    handle_events(fd)?;

    println!("Listening for events stopped.");
    Ok(())
}
