use std::future::Future;
use tokio::runtime::Handle;

pub fn block_on<F: Future>(future: F) -> F::Output {
    let handle = Handle::current();
    handle.block_on(future)
}
