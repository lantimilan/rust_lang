// src/main.rs

// use a thread pool
// requests are enqueued
// each idle thread poll from queue and process request
// so it runs a loop until thread pool shutdown
// while (true)
//   req = queue.poll(); wait until timeout
//   process(req)
//
// alternative designs:
// 1. fork/join
// 2. single threaded async IO model
// 3. multi-thread async IO model
use std::{
    fs,
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        // spawn one thread per request
        thread::spawn(|| {
            handle_connection(stream);
        });

        // println!("Connection established!");
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            // simulate a slow request
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello_sleep.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();
    
    let response = format!("{status_line}\r\nContent-length: {length}\r\n\r\n{contents}");
    
    stream.write_all(response.as_bytes()).unwrap();

    // println!("Request: {http_request:#?}");
}
