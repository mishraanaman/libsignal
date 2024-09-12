// Copyright (C) 2024 Signal Messenger, LLC.
// SPDX-License-Identifier: AGPL-3.0-only
//

use crate::backup::chat::{ChatItemError, Reaction};
use crate::backup::frame::RecipientId;
use crate::backup::method::Lookup;
use crate::backup::serialize::{SerializeOrder, UnorderedList};
use crate::backup::sticker::MessageSticker;
use crate::backup::{TryFromWith, TryIntoWith as _};
use crate::proto::backup as proto;

/// Validated version of [`proto::StickerMessage`].
#[derive(Debug, serde::Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct StickerMessage<Recipient> {
    #[serde(bound(serialize = "Recipient: serde::Serialize + SerializeOrder"))]
    pub reactions: UnorderedList<Reaction<Recipient>>,
    pub sticker: MessageSticker,
    _limit_construction_to_module: (),
}

impl<R: Clone, C: Lookup<RecipientId, R>> TryFromWith<proto::StickerMessage, C>
    for StickerMessage<R>
{
    type Error = ChatItemError;

    fn try_from_with(item: proto::StickerMessage, context: &C) -> Result<Self, Self::Error> {
        let proto::StickerMessage {
            reactions,
            sticker,
            special_fields: _,
        } = item;

        let reactions = reactions
            .into_iter()
            .map(|r| r.try_into_with(context))
            .collect::<Result<_, _>>()?;

        let sticker = sticker
            .into_option()
            .ok_or(ChatItemError::StickerMessageMissingSticker)?
            .try_into()?;

        Ok(Self {
            reactions,
            sticker,
            _limit_construction_to_module: (),
        })
    }
}

#[cfg(test)]
mod test {
    use test_case::test_case;

    use super::*;
    use crate::backup::chat::testutil::TestContext;
    use crate::backup::chat::ReactionError;
    use crate::backup::recipient::FullRecipientData;

    impl proto::StickerMessage {
        pub(crate) fn test_data() -> Self {
            Self {
                reactions: vec![proto::Reaction::test_data()],
                sticker: Some(proto::Sticker::test_data()).into(),
                ..Default::default()
            }
        }
    }

    #[test_case(|x| x.reactions.clear() => Ok(()); "no reactions")]
    #[test_case(|x| x.reactions.push(Default::default()) => Err(ChatItemError::Reaction(ReactionError::EmptyEmoji)); "invalid reaction")]
    fn sticker_message(modifier: fn(&mut proto::StickerMessage)) -> Result<(), ChatItemError> {
        let mut message = proto::StickerMessage::test_data();
        modifier(&mut message);

        message
            .try_into_with(&TestContext::default())
            .map(|_: StickerMessage<FullRecipientData>| ())
    }
}
