use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net,
    time::Duration,
};

pub fn handle_mio_connection(stream: &mut mio::net::TcpStream) -> (String, u16) {
    let buf_reader = BufReader::new(stream);

    // The first line of the request contains data in the below format
    // Method Request-URI HTTP-Version CRLF
    // ex: GET /2 HTTP/1.1

    let http_request = buf_reader
        .lines()
        .take(1)
        .next()
        .map(|line| {
            log::info!("Request {line:?}");
            line
        })
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_default();

    let sleep_time = http_request
        .split(" ")
        .nth(1)
        .map(|uri| &uri[1..])
        .filter(|stripped_uri| !stripped_uri.is_empty())
        .map(|sleep_time| sleep_time.parse::<u16>().unwrap());

    log::warn!("Sleep for {sleep_time:?} ms");

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    (response, sleep_time.unwrap_or(1000))
}

pub fn handle_connection(mut stream: net::TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // The first line of the request contains data in the below format
    // Method Request-URI HTTP-Version CRLF
    // ex: GET /2 HTTP/1.1
    let http_request = buf_reader
        .lines()
        .take(1)
        .next()
        .map(|line| {
            println!("Request {line:?}");
            line
        })
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_default();

    let sleep_time = http_request
        .split(" ")
        .nth(1)
        .map(|uri| &uri[1..])
        .filter(|stripped_uri| !stripped_uri.is_empty())
        .map(|sleep_time| sleep_time.parse::<u16>().unwrap())
        .unwrap_or(500);

    std::thread::sleep(Duration::from_millis(sleep_time.into()));

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}
