// SPDX-License-Identifier: APACHE-2.0 AND MIT
//! `Client` and `Connection` structs
use std::default::Default;

/// A [non-consuming] [Connection] builder.
///
/// [Connection]: struct.Connection.html
/// [non-consuming]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html#non-consuming-builders-(preferred):
pub struct Client {
    props: lapin::ConnectionProperties,
}

impl Client {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
    pub async fn connect(&self, uri: &str) -> crate::Result<Connection> {
        let c = lapin::Connection::connect(uri, self.props.clone())
            .await
            .map_err(crate::Error::from)?;
        Ok(Connection(c))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            props: lapin::ConnectionProperties::default(),
        }
    }
}

/// A [non-consuming] [ProducerBuilder] and [ConsumerBuilder] builder.
///
/// [ProducerBuilder]: ../produce/struct.ProducerBuilder.html
/// [ConsumerBuilder]: ../consume/struct.ConsumerBuilder.html
/// [non-consuming]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html#non-consuming-builders-(preferred):
#[derive(Clone)]
pub struct Connection(lapin::Connection);

impl Connection {
    /// Build a [non-consuming] [ProducerBuilder].
    ///
    /// [ProducerBuilder]: ../consume/struct.ProducerBuilder.html
    /// [non-consuming]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html#non-consuming-builders-(preferred):
    pub fn producer_builder(&self) -> crate::ProducerBuilder {
        crate::ProducerBuilder::new(self.clone())
    }
    /// Build a [non-consuming] [ConsumerBuilder].
    ///
    /// [ConsumerBuilder]: ../consume/struct.ConsumerBuilder.html
    /// [non-consuming]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html#non-consuming-builders-(preferred):
    pub fn consumer_builder(&self) -> crate::ConsumerBuilder {
        crate::ConsumerBuilder::new(self.clone())
    }
    /// channel creates a channel and a queue over the [Connection]
    /// and returns the `Future<Output = <lapin::Channel, lapin::Queue>>`.
    pub async fn channel(
        &self,
        queue: &str,
        opts: lapin::options::QueueDeclareOptions,
        field: lapin::types::FieldTable,
    ) -> crate::Result<(lapin::Channel, lapin::Queue)> {
        let ch = self.0.create_channel().await.map_err(crate::Error::from)?;
        let q = ch
            .queue_declare(queue, opts, field)
            .await
            .map_err(crate::Error::from)?;
        Ok((ch, q))
    }
}
