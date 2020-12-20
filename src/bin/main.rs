use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::fs;
use std::thread;
use std::time::Duration;
use hello_webserver::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    //The incoming method on TcpListener returns an iterator that gives us a sequence of streams
    //(TcpStream iterator)
    //Iterating over incoming is equivalent to calling TcpListener::accept in a loop.
    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();

        println!("Connection established!");
        //handle_connection(stream);
        //
        pool.execute(|| { //should work like thread::spawn
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];// buffer is an array with only 1024 bytes in size; which is enough for basic request
    // that syntax means [0, 0 ... * 1024 times]

    // read is from std::io::Read, becoz TcpStream implemented it
    // TcpStream's read here might change it's internal state, hence need to be mut
    stream.read(&mut buffer).unwrap();


    // from_utf8_lossy takes a slice of bytes &[u8]
    // The “lossy” part of the name indicates the behavior of this function when it sees an invalid UTF-8 sequence: it will replace the invalid sequence with �, the U+FFFD REPLACEMENT CHARACTER
    //println!("Request: {}", String::from_utf8_lossy(&buffer[..]));

    let get = b"GET / HTTP/1.1\r\n"; //byte string
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else if buffer.starts_with(sleep) {
        thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK\r\n\r\n", "sleepy.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!("{}{}", status_line, contents);

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

