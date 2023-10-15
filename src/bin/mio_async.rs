use std::{collections::HashMap, io::Write};

use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
use server::handle_mio_connection;

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

                let response = handle_mio_connection(&mut stream);
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
