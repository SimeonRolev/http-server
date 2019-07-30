#![windows_subsystem = "windows"]

extern crate futures;
extern crate http;
extern crate nix;
extern crate hyper;
extern crate hyper_staticfile;
extern crate webbrowser;
extern crate libc;


// This example serves the docs from `target/doc/`.
//
// Run `cargo doc && cargo run --example doc_server`, then
// point your browser to http://localhost:3000/

use futures::{Async::*, Future, Poll, future};
use http::response::Builder as ResponseBuilder;
use http::{Request, Response, StatusCode, header};
use hyper::Body;
use hyper_staticfile::{Static, StaticFuture};
use std::path::Path;
use nix::unistd::{fork, ForkResult};
use std::io::Error;
use std::process::{Command, Stdio};
use libc::getppid;
use std::str::from_utf8;

/// Future returned from `MainService`.
enum MainFuture {
    Root,
    Static(StaticFuture<Body>),
}

impl Future for MainFuture {
    type Item = Response<Body>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            MainFuture::Root => {
                let res = ResponseBuilder::new()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(header::LOCATION, "/index.html")
                    .body(Body::empty())
                    .expect("unable to build response");
                Ok(Ready(res))
            },
            MainFuture::Static(ref mut future) => {
                future.poll()
            }
        }
    }
}

/// Hyper `Service` implementation that serves all requests.
struct MainService {
    static_: Static,
}

impl MainService {
    fn new() -> MainService {
        MainService {
            static_: Static::new(Path::new(".")),
        }
    }
}

impl hyper::service::Service for MainService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Error;
    type Future = MainFuture;

    fn call(&mut self, req: Request<Body>) -> MainFuture {
        if req.uri().path() == "/close" {
            //eprintln!("Close request accepted");
            std::process::exit(0x0100)
        }

        if req.uri().path() == "/" {
            MainFuture::Root
        } else {
            MainFuture::Static(self.static_.serve(req))
        }
    }
}

/// Application entry point.
/*
fn main() {
    let addr = ([127, 0, 0, 1], 0).into();
    let server = hyper::Server::bind(&addr)
        .serve(|| future::ok::<_, Error>(MainService::new()));

    let bound = server.local_addr();
    let server_url = format!("http://{}:{}", bound.ip(), bound.port());
    eprintln!("Doc server running on {}", server_url);
    
    let url = format!("{}/index.html", server_url);
    webbrowser::open(&url).expect("Error opening url");
    hyper::rt::run(server.map_err(|e| eprintln!("server error: {}", e)));
}
*/

/// Application entry point.
fn main() {
    match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("Continuing execution in parent process, new child has pid: {}", child);
            let command = Command::new("bash")
            .arg("-c")
            .arg(format!("pid=\"{}\"; while [ $pid -ne \"1\" ]; do to_kill=\"$pid\"; echo $pid; pid=$(ps -p $pid -o ppid=); done; kill -9 $to_kill", unsafe { getppid() }))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to execute process");

            let output = command
                .wait_with_output()
                .expect("failed to wait on child");

            let stdout = from_utf8(output.stdout.as_slice()).expect("error convertin from utf-8");
            let stderr = from_utf8(output.stderr.as_slice()).expect("error convertin from utf-8");
            println!("{}", stdout);
            println!("{}", stderr);
        }
        Ok(ForkResult::Child) => {
            // TODO: close stdio
            let addr = ([127, 0, 0, 1], 0).into();
            let server = hyper::Server::bind(&addr)
                .serve(|| future::ok::<_, Error>(MainService::new()));

            let bound = server.local_addr();
            let server_url = format!("http://{}:{}", bound.ip(), bound.port());
            eprintln!("Doc server running on {}", server_url);
            
            let url = format!("{}/index.html", server_url);
            webbrowser::open(&url).expect("Error opening url");
            hyper::rt::run(server.map_err(|e| eprintln!("server error: {}", e)));
        },
        Err(_) => panic!("Fork failed"),
    }
}
