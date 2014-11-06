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

type Mapper<I, O> = proc(I):'static -> O;

trait Join {
    fn join(self);
}

trait Produce<O, J: Join> {
    fn produce_async<C: Consume<O>>(self, consumer: C) -> J;
    fn produce_sync(self) -> O {
        let mut saved = None;
        self.produce_async(SaveConsumer::new(&mut saved)).join();
        saved.unwrap()
    }
}

trait Consume<I> {
    fn consume(self, input: I);
}

struct NoopJoiner;
impl Join for NoopJoiner {
    fn join(self) {}
}

struct ImmediateProducer<O> {
    immediate: O,
} impl<O> ImmediateProducer<O> {
    pub fn new(immediate: O) -> ImmediateProducer<O> {
        ImmediateProducer { immediate: immediate }
    }
} impl<O> Produce<O, NoopJoiner> for ImmediateProducer<O> {
    fn produce_async<C: Consume<O>>(self, consumer: C) -> NoopJoiner {
        consumer.consume(self.immediate);
        NoopJoiner
    }
}

struct SaveConsumer<'s, I: 's> {
    to: &'s mut Option<I>,
} impl<'s, I: 's> SaveConsumer<'s, I> {
    fn new(to: &'s mut Option<I>) -> SaveConsumer<'s, I> {
        SaveConsumer { to: to }
    }
} impl<'s, I: 's> Consume<I> for SaveConsumer<'s, I> {
    fn consume(self, input: I) {
        *self.to = Some(input);
    }
}

struct MapConsumer<I, O, W> {
    wrapped: W,
    mapper: Mapper<I, O>,
} impl<I, O, W: Consume<O>> MapConsumer<I, O, W> {
    // HKT-s would make me SO happy.
    fn new(wrapped: W, mapper: Mapper<I, O>) -> MapConsumer<I, O, W> {
        MapConsumer {
            wrapped: wrapped,
            mapper: mapper,
        }
    }
} impl<I, O, W: Consume<O>> Consume<I> for MapConsumer<I, O, W> {
    fn consume(self, input: I) {
        self.wrapped.consume((self.mapper)(input));
    }
}


struct MapProducer<I, O, J, W> {
    wrapped: W,
    mapper: Mapper<I, O>,
} impl<I, O, J: Join, W: Produce<I, J>> MapProducer<I, O, J, W> {
    fn new(wrapped: W, mapper: Mapper<I, O>) -> MapProducer<I, O, J, W> {
        MapProducer {
            wrapped: wrapped,
            mapper: mapper,
        }
    }
} impl<I, O, J: Join, W: Produce<I, J>> Produce<O, J>
        for MapProducer<I, O, J, W> {
    fn produce_sync(self) -> O {
        (self.mapper)(self.wrapped.produce_sync())
    }
    fn produce_async<C: Consume<O>>(self, consumer: C) -> J {
        self.wrapped.produce_async(MapConsumer::new(consumer, self.mapper))
    }
}




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
