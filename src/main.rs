#[macro_use]
extern crate log;

use daemonize::Daemonize;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::http::StatusCode;
use warp::{reject, Filter, Rejection, Reply};

#[cfg(not(debug_assertions))]
static WEB_ROOT: &str = "/var/sudoku/";
#[cfg(debug_assertions)]
static WEB_ROOT: &str = "./assets/";

static PAGE404: &str = r"
<!DOCTYPE html>
<html>
    <head>
        <title>404 - not found</title>
    </head>
    <body>
        <h1>File not found</h1>
    </body>
</html>
";

async fn p404(_: Rejection) -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        warp::reply::html(PAGE404),
        StatusCode::NOT_FOUND,
    ))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::env_logger::from_env(
        pretty_env_logger::env_logger::Env::default().default_filter_or("warn"),
    )
    .init();

    let daemonize = Daemonize::new()
        .pid_file("/run/sudoku.pid")
        .chown_pid_file(true)
        .user("nobody")
        .group("daemon");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

    let assets = warp::fs::dir(WEB_ROOT)
        .map(|file: warp::filters::fs::File| {
            info!("{:#?}", file);
            file
        })
        .recover(p404);

    let routes = warp::get().and(assets);

    info!("Serving static files from {}", WEB_ROOT);

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9917);

    warp::serve(routes).run(socket).await;
}
