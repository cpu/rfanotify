# Rust fanotify example &emsp; [![Travis Badge]][Travis CI]

[Travis Badge]: https://img.shields.io/travis/com/cpu/rfanotify
[Travis CI]: https://travis-ci.com/cpu/rfanotify

---

* [About](#About)
* [Building](#Building)
* [Usage](#Usage)
* [Example](#Example)
* [Filesystem vs Mount](#Filesystem-vs-Mount)
* [Nix and Libc forks](#Nix-and-Libc-forks)
  * [Updates to the fanotify API](#Updates-to-the-fanotify-API)
* [See Also](#See-Also)

### About

This repository is a Rust port of the Linux [man-pages project]'s [example
C program][fanotify-example] for the Linux `fanotify` API. It offers a simple
program called `rfanotify` that responds to and logs permission requests to
open a file on a monitored filesystem or mount. It also logs when a process
writes a file on a monitored filesystem/mount.

The `fanotify` API is a linux-specific API for notification and interception of
filesystem events. In some ways it is similar to `inotify`, but more powerful.
Unlike `inotify` based approaches to filesystem monitoring using `fanotify` has
several advantages:

* An entire filesystem/mount can be monitored with very low overhead.
* The `pid` of the program causing the filesystem event is known.
* The event handling program can **deny** file opens for the monitored
  filesystem.

For more information, see:

* [the LWN article on `fanotify`][LWN-Article].
* [`man 7 fanotify`][man-fanotify].
* [`man 2 fanotify_init`][man-fanotify_init].
* [`man 2 fanotify_mark`][man-fanotify_mark].

**NOTE: This is my first attempt at writing Rust and it's likely not especially idomatic/clean. Kind PRs/suggestions welcome.**

[man-pages project]: https://www.kernel.org/doc/man-pages/
[fanotify-example]: https://gist.github.com/cpu/b68e7bbdf60619c1cdf1ebc27b1e4ae5
[LWN-Article]: https://lwn.net/Articles/339399/
[man-fanotify]: http://man7.org/linux/man-pages/man7/fanotify.7.html
[man-fanotify_init]: http://man7.org/linux/man-pages/man2/fanotify_init.2.html
[man-fanotify_mark]: http://man7.org/linux/man-pages/man2/fanotify_mark.2.h

### Building

Since this project is written in Rust, you'll need to grab a [Rust
installation] in order to compile it.

`rfanotify` was developed with Rust 1.34.0 (stable). It may work with other
versions, but this isn't guaranteed.

To build `rfanotify`:

```bash
git clone https://github.com/cpu/rfanotify && cd rfanotify
cargo build --release
```

[Rust installation]: https://www.rust-lang.org/learn/get-started

### Usage

After [building from source](#Building), run:

```bash
sudo ./target/release/rfanotify <directory>
```

If no explicit `directory` argument is provided the filesystem/mount of the
current working directory is monitored.

You can avoid running `rfanotify` as `root` by instead giving the binary
the `CAP_SYS_ADMIN` capability, and then running it as a normal user:

```bash
sudo setcap cap_sys_admin+eip target/release/rfanotify
./target/release/rfanotify <directory>
```

The `rfanotify` program adds a `fanotify` watch on the entire filesystem/mount
backing `<directory>`. When a process tries to open a file on the monitored
filesystem/mount a `FAN_OPEN_PERM` event is received and logged by `rfanotify`
and a `FAN_ALLOW` response is returned, allowing the open to complete. When a
process closes a file a `FAN_CLOSE_WRITE` event is received and logged.

Events are logged to stdout including the absolute path of the accessed file
and the program executable performing the access. Both of these values are
retrieved by `fd` using the [Linux procfs][procfs]. If the accessed file has
been deleted since the time the event was generated then the filename will have
"(deleted)" appended by `readlink` and the procfs. If the pid that generated
the access has terminated since the time the event was generated then the
gone pid is printed instead.

[procfs]: http://man7.org/linux/man-pages/man5/proc.5.html

### Example

Here's an example of running `rfanotify` on a fresh Ubuntu 19.04 VM with a Linux
5.0.0-38-generic kernel.

First `rfanotify` is started inside of a `screen` session:

```bash
sudo rfanotify
```

Next, a separate window is created with `ctrl-a c` and a file is edited with vim:

```bash
vim /tmp/test.txt
```

After exiting `vim` and switching back to the first screen window running
`rfanotify` you should see output like:

```bash
FAN_OPEN_PERM: File /usr/lib/x86_64-linux-gnu/utempter/utempter Exe /usr/bin/screen
FAN_OPEN_PERM: File /usr/lib/x86_64-linux-gnu/ld-2.29.so Exe /usr/lib/x86_64-linux-gnu/utempter/utempter
FAN_OPEN_PERM: File /etc/ld.so.cache Exe /usr/lib/x86_64-linux-gnu/utempter/utempter
<snipped>
FAN_CLOSE_WRITE: File /tmp/.test.txt.swx Exe /usr/bin/vim.basic
FAN_CLOSE_WRITE: File /tmp/.test.txt.swp Exe /usr/bin/vim.basic
FAN_OPEN_PERM: File /tmp/.test.txt.swp Exe /usr/bin/vim.basic
FAN_OPEN_PERM: File /usr/share/vim/vim81/scripts.vim Exe /usr/bin/vim.basic
FAN_OPEN_PERM: File /usr/share/vim/vim81/ftplugin/text.vim Exe /usr/bin/vim.basic
FAN_OPEN_PERM: File /tmp/test.txt Exe /usr/bin/vim.basic
FAN_CLOSE_WRITE: File /tmp/test.txt Exe /usr/bin/vim.basic
FAN_CLOSE_WRITE: File /tmp/.test.txt.swp Exe /usr/bin/vim.basic
```

The full program output can be viewed [here][example-output].

[example-output]: https://github.com/cpu/rfanotify/blob/master/rfanotify.eg.output.txt

### Filesystem vs Mount

On Linux kernel versions >= 4.2.0 `rfanotify` uses `fanotify_mark` with the
`FAN_MARK_FILESYSTEM` flag. On older versions `FAN_MARK_MOUNT` is used instead.
When marking a filesystem **mount** instead of a filesystem it is possible
events will be missed.

For example, consider if the device `/dev/sdb1` is mounted to `/mnt/example` as well as having a bind mount to `/mnt/example-b` (e.g. `mount --bind /mnt/example /mnt/example-b`).

If `rfanotify /mnt/example` is run on a Linux kernel version >= 4.2.0 then events will be logged regardless of which mount is used. (e.g editing `/mnt/example/foo.txt` or `/mnt/example-b/foo.txt` will both log `rfanotify` events). The **filesystem** for `/mnt/example` is monitored.

If `rfanotify /mnt/example` is run on a Linux kernel version < 4.2.0 then events will be logged only for the `/mnt/example` mount. (e.g editing `/mnt/example/foo.txt` will log `rfanotify` events but editing `/mnt/example-b/foo.txt` will **not**. Only the **mount** of `/mnt/example` is monitored.

### Nix and Libc forks

While the `fanotify` system calls have been available since Linux 2.6.37 there
are no bindings in the Rust `libc` crate, or any wrappers in the `nix` crate.

This project uses forks of both crates that were extended with the required
types/bindings/wrappers:

* A fork of the `libc` crate is used to [add `fanotify` bindings][libc-fanotify].
* A fork of the `nix` crate [adds safe wrappers][nix-fanotify].

[libc-fanotify]: https://github.com/cpu/libc/commit/8948e46ec45f01e88a3792b1d8c594a9b9c95195
[nix-fanotify]: https://github.com/cpu/nix/commit/d4b52fdfe2219be2c508168b22c8dfcf44d352ce

#### Updates to the fanotify API

The `fanotify` API has been updated several times since it was enabled in Linux
2.6.37. The `rfanotify` code was written using the man page content from
release 4.04 of the Linux [man-pages project] and tested on Linux kernel version
4.4.0 and 5.0.0.

[Linux 4.20] added `FAN_MARK_FILESYSTEM` to "enable monitoring of filesystem
events on all filesystem objects regardless of the mount where event was
generated". It also added `FAN_REPORT_TID` to get thread IDs that triggered
events.

[Linux 5.0] added the ability to watch for when a file is opened with the intent
to execute (`FAN_OPEN_EXEC`).

[Linux 5.1] added many of `inotify`'s directory based events to `fanotify`
(e.g. `FAN_ATTRIB`, `FAN_CREATE`, etc). It also added a new `fanotify_init`
flag `FAN_REPORT_FID` that allows receiving an additional structure beyond the
base `fanotify_event_metadata` structure that contains information about the
filesystem object correlated with an event (e.g. a monitored directory or
mount).

The forked `libc` and `nix` crates used by this project do not (yet) implement
any of these updates (except for `FAN_MARK_FILESYSTEM`). This project does not
(yet) port the `fanotify_fid.c` example C program included in release 5.05 of
the Linux [man-pages project].

[Linux 4.20]: https://kernelnewbies.org/Linux_4.20#Core_.28various.29
[Linux 5.0]: https://kernelnewbies.org/Linux_5.0#Core_.28various.29
[Linux 5.1]: https://kernelnewbies.org/Linux_5.1#Improved_fanotify_for_better_file_system_monitorization

### See Also

Martin Pitt cleverly used the `fanotify` API to develop [`fatrace`][fatrace], a
program for reporting system wide file access events tailored towards reducing
power consumption.

[fatrace]: https://piware.de/2012/02/fatrace-report-system-wide-file-access-events/
