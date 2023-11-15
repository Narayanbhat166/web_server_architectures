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
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

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
            log::info!("Received an event");
            log::debug!("Event details {event:?}");

            match event.token() {
                Token(0) => loop {
                    log::info!("Might be a Server event");
                    let mut stream = match listener.accept() {
                        Ok((stream, _)) => {
                            log::info!("Accepted a connection");
                            stream
                        }
                        Err(err) => {
                            log::error!("{err:?}");
                            break;
                        }
                    };

                    stream
                        .register(poll.registry(), Token(counter), Interest::READABLE)
                        .unwrap();

                    sockets_hm.insert(counter, stream);
                    counter += 1;
                },
                _ => {
                    if event.is_readable() {
                        log::info!("Readable event");

                        let token = event.token().0;
                        let mut stream = sockets_hm.get_mut(&token).unwrap();
                        let (response, sleep_time) = handle_mio_connection(&mut stream);

                        let expire_timeout = get_unix_timestamp() + u128::from(sleep_time);
                        timer_queue.push_back((expire_timeout, token));

                        responses.insert(token, response);
                    } else if event.is_writable() {
                        log::info!("Writable event");

                        let token = event.token().0;

                        let stream = sockets_hm.get_mut(&token).unwrap();
                        let response = responses.get(&token).unwrap();

                        if !event.is_write_closed() {
                            stream
                                .write_all(response.as_bytes())
                                .expect("Could not write to stream");
                            log::info!("Writing Done");

                            stream.flush().unwrap();

                            poll.registry().deregister(stream).unwrap();
                            sockets_hm.remove(&token);
                            responses.remove(&token);
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
                    log::info!("Timer expired for token {token}");

                    // Timer has expired, register interest for writing to the stream
                    let (_, token) = timer_queue.pop_front().unwrap();
                    let tcp_stream = sockets_hm.get_mut(&token).unwrap();

                    log::info!("Registering write interest");
                    tcp_stream
                        .reregister(poll.registry(), Token(token), Interest::WRITABLE)
                        .unwrap();
                } else {
                    // println!("Not expired {event_timestamp} {token}");
                    // later events will not have been expired as per the assuption that
                    // default timeout that this application supports is 1s
                    break;
                }
            }
        }
    }
}
