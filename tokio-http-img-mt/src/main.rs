extern crate bytes;
extern crate futures;
extern crate futures_cpupool;
extern crate httparse;
extern crate magick_rust;
extern crate net2;
extern crate num_cpus;
extern crate time;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

mod date;
mod http;
mod request;
mod response;

use std::io;
use std::str;
use request::Request;
use response::Response;

use magick_rust::{MagickWand, magick_wand_genesis};
use futures::future;
use futures::{BoxFuture, Future};
use futures_cpupool::CpuPool;
use tokio_proto::TcpServer;
use tokio_service::Service;
use std::time::Duration;
use std::thread;

struct Server {
    thread_pool: CpuPool,
}

impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = BoxFuture<Response, io::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let paths: Vec<&str> = req.path().split('/').collect();
        if paths.len() < 4 {
            let mut r = Response::new();
            r.status_code(404, "NOT FOUND");
            return Box::new(future::ok(r));
        }
        let mut w = paths[1].parse::<usize>().unwrap();
        let mut h = paths[2].parse::<usize>().unwrap();
        let mut f = paths[3];
        let filepath = "/tmp/".to_string() + f;
        let img = self.thread_pool.spawn_fn(move || {
            let wand = MagickWand::new();
            match wand.read_image(filepath.as_str()) {
                Ok(b) => (),
                Err(e) => println!("{:?}", e),
            }
            wand.fit(w, h);
            let img = wand.write_image_blob("jpeg").unwrap();
            let res: Result<Vec<u8>, io::Error> = Ok(img);
            res
        });

        img.map(|img| {
                     let mut res = Response::new();
                     res.header("Content-Type", "image/jpeg");
                     res.body_blob(img.as_slice());
                     res
                 })
            .boxed()
    }
}

fn main() {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let thread_pool = CpuPool::new(num_cpus::get() * 8);
    magick_wand_genesis();
    let mut serv = TcpServer::new(http::Http, addr);
    serv.threads(num_cpus::get());
    serv.serve(move || Ok(Server { thread_pool: thread_pool.clone() }))
}
