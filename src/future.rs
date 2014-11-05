use movecell::MoveCell;
use std::rc::Rc;

pub type Chainer<I> = proc(I):'static -> ();

pub trait Fulfiller<T> {
    fn sync(&mut self, promise: Promise<T>);
    fn async(&mut self, promise: Promise<T>);
}

pub struct NoopFulfiller<T>;
impl<T> Fulfiller<T> for NoopFulfiller<T> {
    fn sync(&mut self, _promise: Promise<T>) {}
    fn async(&mut self, _promise: Promise<T>) {}
}

pub struct Promise<T> {
    state: StateRef<T>,
}

impl<T> Clone for Promise<T> {
    fn clone(&self) -> Promise<T> {
        Promise { state: self.state.clone() }
    }
}

impl<T> Promise<T> {
    fn new() -> Promise<T> {
        Promise { state: new_state() }
    }

    fn from_state(state: StateRef<T>) -> Promise<T> {
        Promise {
            state: state,
        }
    }

    fn into_cont(self) -> Future<T> {
        Future { state: self.state }
    }

    pub fn fulfill(self, value: T) -> () {
        let state = self.state;
        match state.take() {
            None => { state.put(Ready(value)); },
            Some(Chained(next)) => next(value),
            _ => unreachable!(),
        }
    }
}

#[must_use]
pub struct SyncFuture<'a, T, F: Fulfiller<T> + 'a> {
    fulfiller: Option<&'a mut F>,
    state: StateRef<T>,
}

impl<'a, T, F: Fulfiller<T>> SyncFuture<'a, T, F> {
    pub fn new(fulfiller: &mut F) -> SyncFuture<T, F> {
        SyncFuture {
            fulfiller: Some(fulfiller),
            state: new_state(),
        }
    }

    pub fn ready(&self) -> bool {
        match self.state.get_ref() {
            Some(&Ready(_)) => true, _ => false
        }
    }

    pub fn sync(self) -> T {
        let promise = self.make_promise();
        match self.fulfiller {
            Some(f) => f.sync(promise), _ => {}
        }
        match self.state.take() {
            Some(Ready(value)) => value,
            _ => panic!("Promise was not fulfilled."),
        }
    }

    pub fn async(self) -> Future<T> {
        let promise = self.make_promise();
        match self.fulfiller {
            Some(f) => f.async(promise), _ => {}
        }
        Future { state: self.state }
    }

    fn make_promise(&self) -> Promise<T> {
        Promise::from_state(self.state.clone())
    }
}

impl<T> SyncFuture<'static, T, NoopFulfiller<T>> {
    pub fn new_ready(value: T) -> SyncFuture<'static, T, NoopFulfiller<T>> {
        SyncFuture {
            fulfiller: None,
            state: Rc::new(MoveCell::from_value(Ready(value))),
        }
    }

    pub fn new_with_promise() ->
            (SyncFuture<'static, T, NoopFulfiller<T>>, Promise<T>) {
        let op = SyncFuture { fulfiller: None, state: new_state() };
        let prom = op.make_promise();
        (op, prom)
    }
}


pub struct Future<T> {
    state: StateRef<T>,
}
impl<T> Future<T> {
    pub fn ready(&self) -> bool {
        match self.state.get_ref() {
            Some(&Ready(_)) => true, _ => false
        }
    }

    pub fn map<U: 'static>(self, through: proc(T):'static -> U) -> Future<U> {
        let promise = Promise::new();
        let cont = promise.clone().into_cont();

        match self.state.take() {
            Some(Ready(x)) => {
                promise.fulfill(through(x));
            },
            None => {
                self.state.put(Chained(proc(from) {
                    promise.fulfill(through(from))
                }));
            }
            _ => unreachable!()
        }

        cont
    }

    pub fn then<U: 'static>(self, next: proc(T):'static -> Future<U>)
            -> Future<U> {
        let promise = Promise::new();
        let cont = promise.clone().into_cont();
        match self.state.take() {
            Some(Ready(from)) => {
                next(from).map(proc(to) { promise.fulfill(to); });
            },
            None => {
                self.state.put(Chained(proc(from) {
                        next(from).map(proc(to) {
                            promise.fulfill(to)
                        });
                }));
            }
            _ => unreachable!()
        }
        cont
    }
}

enum State<T> {
    Ready(T),
    Chained(Chainer<T>),
}

type StateRef<T> = Rc<MoveCell<State<T>>>;

fn new_state<T>() -> StateRef<T> { Rc::new(MoveCell::new()) }


#[cfg(test)]
pub mod test {
    use super::Fulfiller;
    use super::SyncFuture;
    use super::Promise;

    use std::cell::Cell;
    use std::cell::RefCell;
    use std::default::Default;
    use std::rc::Rc;

    struct ConstantFulfiller<T: Clone + Default> {
        constant: T,
        promise: Option<Promise<T>>,
        eager: bool
    }

    impl<T: Clone + Default> ConstantFulfiller<T> {
        pub fn new() -> ConstantFulfiller<T> {
            ConstantFulfiller {
                constant: Default::default(),
                promise: None,
                eager: false,
            }
        }

        pub fn new_eager(eager: bool) -> ConstantFulfiller<T> {
            ConstantFulfiller {
                constant: Default::default(),
                promise: None,
                eager: eager,
            }
        }

        pub fn start(&mut self, constant: T)
                -> SyncFuture<T, ConstantFulfiller<T>> {
            self.constant = constant;
            SyncFuture::new(self)
        }

        pub fn poll(&mut self) {
            if self.eager { return; }
            self.promise.take().unwrap().fulfill(self.constant.clone());
        }
    }

    impl<T: Clone + Default> Fulfiller<T> for ConstantFulfiller<T> {
        fn sync(&mut self, promise: Promise<T>) {
            promise.fulfill(self.constant.clone());
        }

        fn async(&mut self, promise: Promise<T>) {
            if self.eager {
                promise.fulfill(self.constant.clone());
            } else {
                self.promise = Some(promise);
            }
        }
    }

    #[test]
    fn test_copy_sync() {
        let mut ful = ConstantFulfiller::new();
        assert_eq!(ful.start(5u).sync(), 5u);
        assert_eq!(ful.start(6u).sync(), 6u);
    }

    #[test]
    fn test_nocopy_sync() {
        let mut ful = ConstantFulfiller::new();
        assert_eq!(ful.start("yah".to_string()).sync().as_slice(), "yah");
        assert_eq!(ful.start("gah".to_string()).sync().as_slice(), "gah");
    }

    #[test]
    fn test_async() {
        for eager in [false, true].iter() {
            let mut ful = ConstantFulfiller::new_eager(*eager);
            let called_getter : Rc<Cell<bool>> = Rc::new(Cell::new(false));
            let called_setter = called_getter.clone();

            {   let cont = ful.start("foo".to_string()).async();
                assert_eq!(cont.ready(), *eager);
                cont.map(proc(x) {
                    assert_eq!(x.as_slice(), "foo");
                    called_setter.set(true);
                });
            }
            ful.poll();
            assert!(called_getter.get(), "eager: {}", eager);
        }
    }

    #[test]
    fn test_ready() {
        let fut = SyncFuture::new_ready(5u);
        assert!(fut.ready());
        assert_eq!(fut.sync(), 5u);
    }

    #[test]
    fn test_then_map() {
        let mut client = ConstantFulfiller::new();
        let client2 = Rc::new(RefCell::new(ConstantFulfiller::new()));
        let client2_clone = client2.clone();
        let (fut, prom) = SyncFuture::new_with_promise();

        client.start(5u).async()
            .then(proc(x) {
                client2.borrow_mut().start(x + 2u).async()
            })
            .map(proc(x) {
                prom.fulfill(x);
            });
        client.poll();
        client2_clone.borrow_mut().poll();
        assert_eq!(fut.sync(), 7u);
    }

    #[test]
    fn test_then_many() {
        let mut client = ConstantFulfiller::new();
        let client2 = Rc::new(RefCell::new(ConstantFulfiller::new()));
        let client3 = Rc::new(RefCell::new(ConstantFulfiller::new()));
        let client4 = Rc::new(RefCell::new(ConstantFulfiller::new()));
        let client5 = Rc::new(RefCell::new(ConstantFulfiller::new()));
        let client2_clone = client2.clone();
        let client3_clone = client3.clone();
        let client4_clone = client4.clone();
        let client5_clone = client5.clone();
        let (fut, prom) = SyncFuture::new_with_promise();

        client.start(2u)
            .async().then(proc(x) {
                client2.borrow_mut().start(x * 3u).async()
            }).then(proc(x) {
                client3.borrow_mut().start(x * 5u).async()
            }).map(proc(x) {
                x * 7u
            }).then(proc(x) {
                client4.borrow_mut().start(x * 11u).async()
            }).then(proc(x) {
                client5.borrow_mut().start(x * 13u).async()
            }).map(proc(x) {
                prom.fulfill(x);
            });
        client.poll();
        client2_clone.borrow_mut().poll();
        client3_clone.borrow_mut().poll();
        client4_clone.borrow_mut().poll();
        client5_clone.borrow_mut().poll();
        assert_eq!(fut.sync(), 2*3*5*7*11*13);
    }
}
