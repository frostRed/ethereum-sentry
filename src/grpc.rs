use crate::eth::MessageId;
use anyhow::bail;
use std::convert::TryFrom;

pub mod common {
    tonic::include_proto!("common");
}

pub mod control {
    tonic::include_proto!("control");
}

pub mod sentry {
    tonic::include_proto!("sentry");
}

impl TryFrom<MessageId> for control::InboundMessageId {
    type Error = anyhow::Error;

    fn try_from(id: MessageId) -> Result<Self, Self::Error> {
        Ok(match id {
            MessageId::NewBlockHashes => Self::NewBlockHashes,
            MessageId::BlockHeaders => Self::BlockHeaders,
            MessageId::BlockBodies => Self::BlockBodies,
            MessageId::NewBlock => Self::NewBlock,
            other => bail!("Invalid message id: {:?}", other),
        })
    }
}
