use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log;
use mio::{
    event::Source,
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};

use server::handle_mio_connection;

/// Get the current unix timestamp accurate to miliseconds
fn get_unix_timestamp() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_millis()
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug);
    let address = "127.0.0.1:6969";
    let mut listener = TcpListener::bind(address.parse().unwrap()).unwrap();
    log::info!("Listening on http://{address}");

    let mut sockets_hm = HashMap::<usize, TcpStream>::new();
    let mut responses = HashMap::<usize, String>::new();

    let mut poll = Poll::new().unwrap();

    poll.registry()
        .register(&mut listener, Token(0), Interest::READABLE)
        .unwrap();

    // A queue to register timeouts
    let mut timer_queue = VecDeque::<(u128, usize)>::new();

    // Can handle 8 events at once
    let mut events = Events::with_capacity(10240);
    let mut counter = 1;
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(10)))
            .unwrap();
        for event in &events {
            println!("{event:?}");
            match event.token() {
                Token(0) => loop {
                    let mut stream = match listener.accept() {
                        Ok((stream, _)) => stream,
                        Err(err) => {
                            eprint!("{err:?}");
                            break;
                        }
                    };
                    println!("Read {counter}");

                    stream
                        .register(poll.registry(), Token(counter), Interest::READABLE)
                        .unwrap();

                    sockets_hm.insert(counter, stream);
                    counter += 1;
                },
                _ => {
                    if event.is_readable() {
                        let token = event.token().0;
                        let mut stream = sockets_hm.get_mut(&token).unwrap();
                        let response = handle_mio_connection(&mut stream);

                        // log::error!("writable event registered for token {token} after 1 seconds");
                        let expire_timeout = get_unix_timestamp() + 500;
                        timer_queue.push_back((expire_timeout, token));

                        responses.insert(token, response);
                    } else if event.is_writable() {
                        let token = event.token().0;
                        // prinstln!("writable event triggered for token {token}");

                        let stream = sockets_hm.get_mut(&token).unwrap();
                        let response = responses.get(&token).unwrap();

                        if !event.is_write_closed() {
                            // println!("Writing to stream");
                            stream
                                .write_all(response.as_bytes())
                                .expect("Could not write to stream");
                            stream.flush().unwrap();

                            poll.registry().deregister(stream).unwrap();
                            sockets_hm.remove(&token);
                            responses.remove(&token);
                            // println!("Writing done!");
                        }
                    }
                }
            }
        }
        // Check if any timer has expired
        if !timer_queue.is_empty() {
            let current_timestamp = get_unix_timestamp();
            while let Some((event_timestamp, token)) = timer_queue.front() {
                if event_timestamp < &current_timestamp {
                    println!("Expired {token}");
                    // Timer has expired, register interest for writing to the stream
                    let (_, token) = timer_queue.pop_front().unwrap();
                    let tcp_stream = sockets_hm.get_mut(&token).unwrap();
                    tcp_stream
                        .reregister(poll.registry(), Token(token), Interest::WRITABLE)
                        .unwrap();
                } else {
                    println!("Not expired {event_timestamp} {token}");
                    // later events will not have been expired as per the assuption that
                    // default timeout that this application supports is 1s
                    break;
                }
            }
        }
    }
}
