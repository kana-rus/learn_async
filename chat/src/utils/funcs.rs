use async_std::{
    task,
    future::Future,
};
use crate::utils::types::Result;


pub(crate) fn spawn_with_loging_error<F>(f: F) -> task::JoinHandle<()>
    where F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async {
        if let Err(e) = f.await {
            eprintln!("{e}")
        }
    })
}
