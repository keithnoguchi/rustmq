// SPDX-License-Identifier: GPL-2.0
use crate::{msg, Client};
use futures_util::stream::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Channel, Result};
use std::default::Default;

pub struct Consumer {
    pub channel: Channel,
    pub consumer: lapin::Consumer,
}

impl Consumer {
    pub async fn run(&mut self) -> Result<()> {
        while let Some(delivery) = &self.consumer.next().await {
            let delivery = delivery.as_ref().unwrap();
            let msg = msg::get_root_as_message(&delivery.data);
            if let Some(reply_to) = delivery.properties.reply_to() {
                self.publish(reply_to.as_str()).await?;
            } else {
                print!("{}", msg.msg().unwrap());
            }
            if let Err(err) = self
                .channel
                .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
                .await
            {
                return Err(err);
            }
        }
        Ok(())
    }
    pub async fn publish(&mut self, queue: &str) -> Result<()> {
        print!("{}", queue);
        Ok(())
    }
}

pub struct ConsumerBuilder {
    pub queue_options: QueueDeclareOptions,
    client: Option<Client>,
}

impl ConsumerBuilder {
    pub fn new(c: Client) -> Self {
        Self {
            client: Some(c),
            ..Default::default()
        }
    }
    pub async fn consumer(&mut self, queue: &str) -> Result<Consumer> {
        let (channel, q) = match self
            .client
            .as_ref()
            .unwrap()
            .channel_and_queue(queue, self.queue_options.clone(), FieldTable::default())
            .await
        {
            Ok((ch, q)) => (ch, q),
            Err(err) => return Err(err),
        };
        let consumer = match channel
            .clone()
            .basic_consume(
                &q,
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
        {
            Ok(c) => c,
            Err(err) => return Err(err),
        };
        Ok(Consumer { channel, consumer })
    }
}

impl Default for ConsumerBuilder {
    fn default() -> Self {
        Self {
            queue_options: QueueDeclareOptions::default(),
            client: None,
        }
    }
}
