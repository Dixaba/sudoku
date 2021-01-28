#[macro_use]
extern crate log;

use rusqlite::{params, Connection, Result, NO_PARAMS};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{fs, io};
use warp::{http::Uri, Filter};

#[derive(Debug)]
struct FileClasses {
    id: i32,
    name: String,
    viewed: u32,
    oof: u32,
    ooyoy: u32,
    aba: u32,
}

static WEB_ROOT: &str = "./assets/";

fn fill_data(data: Option<HashMap<String, String>>) -> String {
    let conn = Connection::open("./db.sqlite").unwrap();

    if let Some(x) = data {
        if !x.is_empty() {
            info!("Got user response {:?}", x);

            let response = FileClasses {
                id: 0,
                name: x.get("filename").expect("Empty shiet").to_string(),
                oof: x.get("oof").is_some() as u32,
                ooyoy: x.get("ooyoy").is_some() as u32,
                aba: x.get("aba").is_some() as u32,

                viewed: 1,
            };

            conn.execute(
                "INSERT INTO files(name, oof, ooyoy, aba, viewed) VALUES(?1, ?2, ?3, ?4, 1)
                    ON CONFLICT(name) DO UPDATE SET oof=oof+?2, ooyoy=ooyoy+?3, aba=aba+?4, viewed=viewed+1",
                params![response.name, response.oof, response.ooyoy, response.aba],
            )
            .unwrap();
        };
    };

    let mut sel = conn.prepare("SELECT * FROM files").unwrap();
    let files_iter = sel
        .query_map(params![], |row| {
            Ok(FileClasses {
                id: row.get(0).unwrap(),
                name: row.get(1).unwrap(),
                viewed: row.get(2).unwrap(),
                oof: row.get(3).unwrap(),
                ooyoy: row.get(4).unwrap(),
                aba: row.get(5).unwrap(),
            })
        })
        .unwrap();

    let mut out = String::new();

    for file in files_iter {
        out = format!("{}{:?}\n", out, file.unwrap());
    }

    out
}

#[tokio::main]
async fn main() {
    pretty_env_logger::env_logger::from_env(
        pretty_env_logger::env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let conn = Connection::open("./db.sqlite").unwrap();
    conn.execute(
        "CREATE TABLE files (
            id      INTEGER PRIMARY KEY,
            name    VARCHAR(255) UNIQUE,
            viewed  INTEGER,
            oof     INTEGER,
            ooyoy   INTEGER,
            aba     INTEGER
        )",
        params![],
    )
    .unwrap_or_default();

    let entries = fs::read_dir(WEB_ROOT.to_owned() + "audios")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    for entry in entries {
        let fname = entry.to_str().unwrap();
        let fname = fname.replace(WEB_ROOT, "");
        info!("Found file {:?}", fname);

        conn.execute(
            "INSERT OR IGNORE INTO files(name, oof, ooyoy, aba, viewed) VALUES(?1,0,0,0,0)",
            &[fname],
        )
        .unwrap();
    }

    conn.close().unwrap();

    let assets = warp::fs::dir(WEB_ROOT).map(|file: warp::filters::fs::File| {
        info!("{:#?}", file);
        file
    });

    let form =
        warp::path("form")
            .and(warp::body::form())
            .map(|form_data: HashMap<String, String>| {
                let mut file = File::open(WEB_ROOT.to_owned() + "index.html").unwrap();
                let mut file_data = String::new();
                file.read_to_string(&mut file_data).unwrap();

                let data = file_data.replace("!!!", &fill_data(Some(form_data)));
                let conn = Connection::open("./db.sqlite").unwrap();
                let least_viewed: String = conn
                    .query_row(
                        "SELECT name FROM files ORDER BY viewed ASC",
                        NO_PARAMS,
                        |row| row.get(0),
                    )
                    .unwrap();

                info!("Selected file {:?} as least viewed", least_viewed);

                let data = data.replace("###", &least_viewed);
                conn.close().unwrap();

                warp::reply::html(data)
            });

    let index = warp::path::end().map(|| warp::redirect(Uri::from_static("/form")));
    let gets = warp::get().and(index.or(form).or(assets));
    let posts = warp::post().and(form);

    let routes = gets.or(posts);

    info!("Serving static files from {}", WEB_ROOT);

    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9917);

    warp::serve(routes).run(socket).await;
}
