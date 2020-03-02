/*
 * Rust "port" of the `man fanotify` example program offered as part of release 4.04 of the Linux
 * man-pages project.
 */

use nix::sys::fanotify::*;
use nix::unistd::close;
use std::env;
use std::fs;

fn handle_events(fd: Fanotify) -> Result<(), Box<dyn std::error::Error>> {
    /* Loop while events can be read from fanotify file descriptor */
    loop {
        /* Read some events */
        /* e.fd contains either None, indicating a queue overflow, or a file descriptor (a
         * nonnegative integer). Here we simply ignore queue overflow (by filtering out cases where
         * `e.fd` is None). */
        for e in fd.read_events()?.iter().filter(|e| e.fd.is_some()) {
            // NOTE(@cpu): safe to unconditionally unwrap here because of `filter` above.
            let event_fd = e.fd.unwrap();

            if e.mask.contains(MaskFlags::FAN_OPEN_PERM) {
                print!("FAN_OPEN_PERM: ");

                /* Allow file to be opened */
                fd.respond(FanotifyResponse {
                    fd: event_fd,
                    response: FanotifyPermissionResponse::FAN_ALLOW,
                })?;
            }

            if e.mask.contains(MaskFlags::FAN_CLOSE_WRITE) {
                print!("FAN_CLOSE_WRITE: ");
            }

            /* Retrieve and print pathname of the accessed file */
            let procfd_path = fs::read_link(format!("/proc/self/fd/{}", event_fd))?;
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

            /* Close the file descriptor of the event */
            if let Err(err) = close(event_fd) {
                eprintln!("err closing fd {}: {}", event_fd, err);
            }

            /* Advance to the next event */
        }
    }
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

    /* Mark the mount for:
     * - permission events before opening files
     * - notification events after closing a write-enabled file descriptor
     */
    fd.mark(
        MarkFlags::FAN_MARK_ADD | MarkFlags::FAN_MARK_MOUNT,
        MaskFlags::FAN_OPEN_PERM | MaskFlags::FAN_CLOSE_WRITE,
        AT_FDCWD,
        input_path.as_str(),
    )?;

    /* Fanotify events may be available */
    handle_events(fd)?;

    println!("Listening for events stopped.");
    Ok(())
}
