// Copyright (C) 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//

use crate::backup::chat::quote::{Quote, QuoteError};
use crate::backup::chat::{Reaction, ReactionError};
use crate::backup::file::{MessageAttachment, MessageAttachmentError};
use crate::backup::frame::RecipientId;
use crate::backup::method::Lookup;
use crate::backup::serialize::{SerializeOrder, UnorderedList};
use crate::backup::{TryFromWith, TryIntoWith as _};
use crate::proto::backup as proto;

/// Validated version of a voice message [`proto::StandardMessage`].
#[derive(Debug, serde::Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct VoiceMessage<Recipient> {
    pub quote: Option<Quote<Recipient>>,
    #[serde(bound(serialize = "Recipient: serde::Serialize + SerializeOrder"))]
    pub reactions: UnorderedList<Reaction<Recipient>>,
    pub attachment: MessageAttachment,
    _limit_construction_to_module: (),
}

#[derive(Debug, displaydoc::Display, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum VoiceMessageError {
    /// attachment: {0}
    Attachment(#[from] MessageAttachmentError),
    /// has unexpected field {0}
    UnexpectedField(&'static str),
    /// has {0} attachments
    WrongAttachmentsCount(usize),
    /// attachment should be a VOICE_MESSAGE, but was {0:?}
    WrongAttachmentType(proto::message_attachment::Flag),
    /// invalid quote: {0}
    Quote(#[from] QuoteError),
    /// invalid reaction: {0}
    Reaction(#[from] ReactionError),
}

impl<R: Clone, C: Lookup<RecipientId, R>> TryFromWith<proto::StandardMessage, C>
    for VoiceMessage<R>
{
    type Error = VoiceMessageError;

    fn try_from_with(item: proto::StandardMessage, context: &C) -> Result<Self, Self::Error> {
        let proto::StandardMessage {
            quote,
            reactions,
            text,
            attachments,
            linkPreview,
            longText,
            special_fields: _,
        } = item;

        match () {
            _ if text.is_some() => Err("text"),
            _ if longText.is_some() => Err("longText"),
            _ if !linkPreview.is_empty() => Err("linkPreview"),
            _ => Ok(()),
        }
        .map_err(VoiceMessageError::UnexpectedField)?;

        let [attachment] = <[_; 1]>::try_from(attachments)
            .map_err(|attachments| VoiceMessageError::WrongAttachmentsCount(attachments.len()))?;

        let attachment: MessageAttachment = attachment.try_into()?;

        if attachment.flag != proto::message_attachment::Flag::VOICE_MESSAGE {
            return Err(VoiceMessageError::WrongAttachmentType(attachment.flag));
        }

        let quote = quote
            .into_option()
            .map(|q| q.try_into_with(context))
            .transpose()?;
        let reactions = reactions
            .into_iter()
            .map(|r| r.try_into_with(context))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            reactions,
            quote,
            attachment,
            _limit_construction_to_module: (),
        })
    }
}

#[cfg(test)]
mod test {
    use test_case::test_case;

    use super::*;
    use crate::backup::chat::testutil::TestContext;
    use crate::backup::recipient::FullRecipientData;

    #[test]
    fn valid_voice_message() {
        assert_eq!(
            proto::StandardMessage::test_voice_message_data()
                .try_into_with(&TestContext::default()),
            Ok(VoiceMessage {
                quote: Some(Quote::from_proto_test_data()),
                reactions: vec![Reaction::from_proto_test_data()].into(),
                attachment: MessageAttachment::from_proto_voice_message_data(),
                _limit_construction_to_module: ()
            })
        )
    }

    #[test_case(|x| x.reactions.clear() => Ok(()); "no reactions")]
    #[test_case(|x| x.reactions.push(proto::Reaction::default()) => Err(VoiceMessageError::Reaction(ReactionError::EmptyEmoji)); "invalid reaction")]
    #[test_case(|x| x.quote = None.into() => Ok(()); "no quote")]
    #[test_case(|x| x.attachments.clear() => Err(VoiceMessageError::WrongAttachmentsCount(0)); "no attachments")]
    #[test_case(|x| x.attachments.push(proto::MessageAttachment::default()) => Err(VoiceMessageError::WrongAttachmentsCount(2)); "extra attachment")]
    fn voice_message(modifier: fn(&mut proto::StandardMessage)) -> Result<(), VoiceMessageError> {
        let mut message = proto::StandardMessage::test_voice_message_data();
        modifier(&mut message);

        message
            .try_into_with(&TestContext::default())
            .map(|_: VoiceMessage<FullRecipientData>| ())
    }
}
