use std::{
    collections::HashMap,
    fs,
    future::Ready,
    io::{BufRead, BufReader, Write},
    thread::sleep,
    time::Duration,
};

use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};

fn main() {
    let address = "127.0.0.1:6969";
    let mut listener = TcpListener::bind(address.parse().unwrap()).unwrap();

    let mut sockets_hm = HashMap::<usize, TcpStream>::new();
    let mut responses = HashMap::<usize, String>::new();

    let mut poll = Poll::new().unwrap();

    poll.registry()
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();

    let mut events = Events::with_capacity(8);
    let mut counter = 1;
    loop {
        poll.poll(&mut events, None).unwrap();
        for event in &events {
            println!("{event:?}");
            if event.is_readable() {
                let mut stream = listener.accept().unwrap().0;

                let response = handle_connection(&mut stream);
                counter += 1;

                println!("writable event registered for token {counter}");
                poll.registry()
                    .register(&mut stream, Token(counter), Interest::WRITABLE)
                    .unwrap();

                sockets_hm.insert(counter, stream);
                responses.insert(counter, response);
            } else if event.is_writable() {
                let token = event.token().0;
                println!("writable event triggered for token {token}");

                let stream = sockets_hm.get_mut(&token).unwrap();
                let response = responses.get(&token).unwrap();

                if !event.is_write_closed() {
                    println!("Writing to stream");
                    stream
                        .write_all(response.as_bytes())
                        .expect("Could not write to stream");
                    stream.flush().unwrap();

                    poll.registry().deregister(stream).unwrap();
                    sockets_hm.remove(&token);
                    responses.remove(&token);
                    println!("Writing done!");
                }
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream) -> String {
    // let buf_reader = BufReader::new(stream);

    // The first line of the request contains data in the below format
    // Method Request-URI HTTP-Version CRLF
    // ex: GET /2 HTTP/1.1
    // let http_request = buf_reader
    //     .lines()
    //     .take(1)
    //     .next()
    //     .map(|line| {
    //         println!("Request {line:?}");
    //         line
    //     })
    //     .transpose()
    //     .ok()
    //     .flatten()
    //     .unwrap_or_default();

    // let sleep_time = http_request
    //     .split(" ")
    //     .nth(1)
    //     .map(|uri| &uri[1..])
    //     .filter(|stripped_uri| !stripped_uri.is_empty())
    //     .map(|sleep_time| sleep_time.parse::<u16>().unwrap())
    //     .unwrap_or(100);

    // std::thread::sleep(Duration::from_millis(sleep_time.into()));

    let status_line = "HTTP/1.1 200 OK";
    let contents = fs::read_to_string("static/index.html").unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    response
}
