use aws_sdk_sqs::{Client, types::Message};
use uuid::Uuid;

pub struct Queue {
    client: aws_sdk_sqs::Client,
    validation_url: String,
    process_url: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidationMessage {
    pub recording_id: Uuid,
    pub version: u8,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProcessingMessage {
    pub recording_id: Uuid,
    pub version: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Failed sending message")]
    Send,

    #[error("Failed receving messages")]
    Receive,
}

impl Queue {
    pub fn new(client: Client, validation_url: String, process_url: String) -> Self {
        Self {
            client,
            validation_url,
            process_url,
        }
    }

    pub async fn emit_validation(self: &Queue, msg: &str) -> Result<(), QueueError> {
        let _ = self
            .client
            .send_message()
            .queue_url(&self.validation_url)
            .message_body(msg)
            .send()
            .await
            .map_err(|_| QueueError::Send)?;

        Ok(())
    }

    pub async fn queue_processing(self: &Queue, msg: &str) -> Result<(), QueueError> {
        let _ = self
            .client
            .send_message()
            .queue_url(&self.process_url)
            .message_body(msg)
            .send()
            .await
            .map_err(|_| QueueError::Send)?;

        Ok(())
    }

    pub async fn receive_message(self: &Queue) -> Result<Option<Message>, QueueError> {
        let response = self
            .client
            .receive_message()
            .queue_url(&self.validation_url)
            .max_number_of_messages(1)
            .wait_time_seconds(20)
            .send()
            .await
            .map_err(|_| QueueError::Receive)?;

        Ok(response
            .messages
            .and_then(|messages| messages.into_iter().next()))
    }

    pub async fn receieve_process_message(&self) -> Result<Option<Message>, QueueError> {
        let response = self
            .client
            .receive_message()
            .queue_url(&self.process_url)
            .max_number_of_messages(1)
            .wait_time_seconds(20)
            .send()
            .await
            .map_err(|_| QueueError::Receive)?;

        Ok(response
            .messages
            .and_then(|messages| messages.into_iter().next()))
    }

    pub async fn delete_validation(&self, message: &Message) -> Result<(), QueueError> {
        let receipt_handle = message.receipt_handle().ok_or_else(|| QueueError::Send)?;

        self.client
            .delete_message()
            .queue_url(&self.validation_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|_| QueueError::Send)?;

        Ok(())
    }

    pub async fn delete_processing(&self, message: &Message) -> Result<(), QueueError> {
        let receipt_handle = message.receipt_handle().ok_or_else(|| QueueError::Send)?;

        self.client
            .delete_message()
            .queue_url(&self.process_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|_| QueueError::Send)?;

        Ok(())
    }

    pub async fn move_message_dql(&self, message: &Message) -> Result<(), QueueError> {
        Ok(())
    }
}
