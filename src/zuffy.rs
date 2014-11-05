#![feature(unboxed_closures, unboxed_closure_sugar)]
#![feature(phase)]
#![feature(overloaded_calls)]
#![feature(slicing_syntax)]

#[phase(plugin, link)]
extern crate log;
extern crate time;
extern crate zmq;


pub mod error;
pub mod future;
pub mod lazy;
pub mod movecell;
pub mod reactor;


#[cfg(not(test))]
fn main() {
    use zmq;
    use reactor::Reactor;

    let mut ctx = zmq::Context::new();
    let mut server = ctx.socket(zmq::DEALER).unwrap();
    server.bind("tcp://*:8080").unwrap();

    let sf = StdFuture::spawn(proc() {
        let mut reactor = Reactor::new();
        reactor.push_item(
            server.as_poll_item(zmq::POLLIN),
            || {
                info!("server: received '{}'", server.recv_str(0).unwrap());
            });
        reactor.run();
    });

    let mut client = ctx.socket(zmq::DEALER).unwrap();
    client.connect("tcp://127.0.0.1:8080").unwrap();
    client.send_str("message1", 0).unwrap();
    client.send_str("message2", 0).unwrap();
    client.send_str("message3", 0).unwrap();
    client.send_str("message4", 0).unwrap();
    client.send_str("message5", 0).unwrap();

    sf.unwrap();
}
