use std::net::TcpListener;

/// This server is run on a single thread, does blocking IO
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Started the server");
    for stream in listener.incoming() {
        let stream = stream.unwrap();

        server::handle_connection(stream);
    }
}
