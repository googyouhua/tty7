#![cfg(unix)]

//! End-to-end through the real router into a real container.
//!
//! Runs only when `TTY7_DOCKER_TEST_IMAGE` names an image already on the
//! machine (and `docker` reaches a daemon). The point is the whole path, not a
//! unit: `RouteHeader::docker` → the router installs `tty7-server` into the
//! container → `docker exec … --stdio` → control handshake → file round trip →
//! the bridge exits when the link closes.
//!
//! ```text
//! DOCKER_HOST=ssh://root@192.168.1.25 \
//! TTY7_BUNDLED_SERVER_DIR=/tmp/tty7-bundle \
//! TTY7_DOCKER_TEST_IMAGE=node:alpine \
//! cargo test -p tty7-server --test docker_route -- --nocapture
//! ```
//!
//! `TTY7_BUNDLED_SERVER_DIR` must hold the server this build produced, named
//! `tty7-server-linux-x86_64-musl`; without it the installer would download a
//! published build instead of the one under test.

use std::io::Write as _;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;
use std::time::{Duration, Instant};

use std::sync::Arc;

use tty7_core::daemon::control::ControlHello;
use tty7_core::daemon::install::{InstallConfirm, InstallDecision};
use tty7_core::daemon::router::{ROUTE_KIND, RemoteRouter, RouteAck, RouteHeader};
use tty7_core::host::Host as _;
use tty7_core::host::remote::RemoteHost;

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .expect("docker should be on PATH")
}

fn container_that_gets_removed(image: &str) -> String {
    let name = format!("tty7-e2e-{}", std::process::id());
    let _ = docker(&["rm", "-f", &name]);
    let out = docker(&[
        "run",
        "-d",
        "--name",
        &name,
        "--entrypoint",
        "sleep",
        image,
        "infinity",
    ]);
    assert!(
        out.status.success(),
        "could not start the test container: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    name
}

/// Any `tty7-server --stdio` process still inside the container. The daemon the
/// bridge starts is `--daemon` and is expected to stay: only the bridge is a
/// leak (spike 1.2).
fn bridges_inside(container: &str) -> Vec<String> {
    let script = "for p in /proc/[0-9]*; do \
                    [ \"$(cat \"$p/comm\" 2>/dev/null)\" = tty7-server ] || continue; \
                    a=$(tr '\\0' ' ' < \"$p/cmdline\" 2>/dev/null); \
                    case \"$a\" in *--stdio*) echo \"${p#/proc/}\";; esac; \
                  done";
    let out = docker(&["exec", container, "sh", "-c", script]);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}


/// Read until the route ack, dropping the progress frames an install emits.
fn read_ack_skipping_prompts(sock: &mut UnixStream) -> std::io::Result<RouteAck> {
    loop {
        let (kind, payload) = tty7_core::daemon::protocol::read_frame(&mut *sock)?;
        if kind == tty7_core::daemon::router::ROUTE_PROMPT_KIND {
            continue;
        }
        let mut framed = Vec::new();
        tty7_core::daemon::protocol::write_frame(&mut framed, kind, &payload)?;
        return RouteAck::read(&mut &framed[..]);
    }
}

#[test]
fn a_container_is_reachable_through_the_router() {
    let Ok(image) = std::env::var("TTY7_DOCKER_TEST_IMAGE") else {
        eprintln!("skipping: TTY7_DOCKER_TEST_IMAGE is not set");
        return;
    };

    let container = container_that_gets_removed(&image);
    let cleanup = Cleanup(container.clone());

    // Put the server in place the way a finished install would leave it, so the
    // route exercises the transport rather than the installer (which the spikes
    // covered). Without this the route would ask for consent and then fetch a
    // published build over the network.
    let bundle = std::env::var("TTY7_BUNDLED_SERVER_DIR")
        .expect("TTY7_BUNDLED_SERVER_DIR must point at the server this build produced");
    let binary = format!("{bundle}/tty7-server-linux-x86_64-musl");
    let target = "/root/.local/share/tty7/bin/tty7-server-c12p6";
    let out = docker(&["exec", &container, "mkdir", "-p", "/root/.local/share/tty7/bin"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let out = docker(&["cp", &binary, &format!("{container}:{target}")]);
    assert!(
        out.status.success(),
        "could not place the server in the container: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = docker(&["exec", &container, "chmod", "755", target]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let dir = tempfile::TempDir::new().unwrap();
    let hub = dir.path().join("hub.sock");
    let listener = UnixListener::bind(&hub).unwrap();

    let _routed = container.clone();
    let router = std::thread::spawn(move || {
        // The container has no server yet, so the route asks before writing
        // one. The grant is scoped to this thread because that is where the
        // router consults it.
        struct Approve;
        impl InstallConfirm for Approve {
            fn confirm(
                &self,
                _request: &tty7_core::daemon::install::InstallRequest,
            ) -> InstallDecision {
                InstallDecision::Approve
            }
        }
        tty7_core::daemon::install::with_install_confirm(Arc::new(Approve), || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = stream.try_clone().unwrap();
            let (kind, payload) = tty7_core::daemon::protocol::read_frame(&mut reader).unwrap();
            assert_eq!(kind, ROUTE_KIND);
            let header = RouteHeader::decode(&payload).unwrap();
            RemoteRouter::route(stream, &header)
        })
    });

    let mut sock = UnixStream::connect(&hub).unwrap();
    let header = RouteHeader::docker(container.clone());
    header.write(&mut sock).unwrap();
    sock.flush().unwrap();

    // The router narrates an install on the same socket: progress frames arrive
    // as `ROUTE_PROMPT_KIND` and carry no answer from this side. Only the ack
    // ends the wait.
    let _ack = read_ack_skipping_prompts(&mut sock)
        .expect("the route into the container should be accepted");

    let hello = ControlHello::host_rpc("docker-route-test", "localhost");
    let host = RemoteHost::over_unix(sock, "routed:docker", &hello)
        .expect("the server inside the container should answer the handshake");

    // The file must be the container's, not this machine's: the same path is
    // absent here, which is what proves which filesystem the route landed on.
    let remote = format!("/tmp/through-docker-{}.txt", std::process::id());
    assert!(
        !std::path::Path::new(&remote).exists(),
        "the probe path already exists on this machine, so it proves nothing"
    );
    let file = host.join(std::path::Path::new("/tmp"), &format!("through-docker-{}.txt", std::process::id()));
    host.write_file(&file, b"a container, not a host").unwrap();
    assert_eq!(
        host.read_file(&file, 1024).unwrap(),
        b"a container, not a host"
    );
    assert!(
        !std::path::Path::new(&remote).exists(),
        "the file was written on this machine instead of in the container"
    );

    drop(host);
    let _ = router.join();

    // The bridge is the per-connection process, and it must be gone shortly
    // after the link closes. The daemon it started stays by design.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let left = bridges_inside(&container);
        if left.is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the bridge was still running 5s after the link closed: {left:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    drop(cleanup);
}

struct Cleanup(String);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = docker(&["rm", "-f", &self.0]);
    }
}
