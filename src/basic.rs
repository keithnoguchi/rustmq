// SPDX-License-Identifier: GPL-2.0
// https://tokio.rs/docs/futures/basic/
use futures;

pub struct HelloWorld;

impl futures::Future for HelloWorld {
    type Item = String;
    type Error = ();
    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        Ok(futures::Async::Ready("hello world".to_string()))
    }
}

pub struct Display<T>(pub T);

impl<T> futures::Future for Display<T>
where
    T: futures::Future,
    T::Item: std::fmt::Display,
{
    type Item = ();
    type Error = T::Error;

    fn poll(&mut self) -> futures::Poll<(), T::Error> {
        const NAME: &str = "basic::Display";
        let value = match self.0.poll() {
            Ok(futures::Async::Ready(value)) => value,
            Ok(futures::Async::NotReady) => return Ok(futures::Async::NotReady),
            Err(err) => return Err(err),
        };
        println!("[{}]: {}", NAME, value);
        Ok(futures::Async::Ready(()))
    }
}

pub struct BetterDisplay<T>(pub T);

impl<T> futures::Future for BetterDisplay<T>
where
    T: futures::Future,
    T::Item: std::fmt::Display,
{
    type Item = ();
    type Error = T::Error;

    fn poll(&mut self) -> futures::Poll<(), T::Error> {
        const NAME: &str = "basic::BetterDisplay";
        let value = futures::try_ready!(self.0.poll());
        println!("[{}]: {}", NAME, value);
        Ok(futures::Async::Ready(()))
    }
}

#[cfg(test)]
mod tests {
    use tokio;
    #[test]
    fn run_hello_display() {
        let fut = super::Display(super::HelloWorld);
        tokio::run(fut);
    }
    #[test]
    fn run_hello_better_display() {
        let fut = super::BetterDisplay(super::HelloWorld);
        tokio::run(fut);
    }
}
