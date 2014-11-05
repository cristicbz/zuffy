use zmq;
use zmq::PollItem;

pub type ReadHandler<'a> = || : 'a -> ();

pub struct Reactor<'a, 'b> {
    readers: Vec<ReadHandler<'a>>,
    poll_set: Vec<zmq::PollItem<'b>>,
}

impl<'a, 'b> Reactor<'a, 'b> {
    pub fn new() -> Reactor<'a, 'b> {
        Reactor {
            readers: Vec::new(),
            poll_set: Vec::new()
        }
    }

    pub fn push_item(&mut self,
                     poll_item: zmq::PollItem,
                     handler: ReadHandler<'a>) {
        self.readers.push(handler);
        self.poll_set.push(poll_item);
    }

    pub fn run(&mut self) {
        loop {
            self.poll();
        }
    }

    fn poll(&mut self) {
        zmq::poll(self.poll_set[mut], -1).unwrap();
        for (index, &item) in self.poll_set.iter().enumerate() {
            if item.get_revents() | zmq::POLLIN != 0 {
                (*&mut self.readers[index])();
            }
        }
    }
}
