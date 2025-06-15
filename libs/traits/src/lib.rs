// Copyright 2025 The Kippu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use ink::{
    codegen::StaticEnv,
    scale::{Decode, Encode},
    EnvAccess,
};
use ink_env::Environment;
use kreivo_apis::{
    apis::{KreivoAPI, ListingsInventoriesAPI, ListingsItemsAPI},
    KreivoApi, KreivoApiEnvironment,
};
use ticketto_types::{
    Error, EventAttribute, EventId, EventInfo, EventState, TicketId, TickettoAttribute,
    DEFAULT_CLASS,
};

const TICKETTO_ATTR: [u8; 13] = *b"TICKETTO_ATTR";

type Balance = <KreivoApiEnvironment as Environment>::Balance;

/// Since we need to keep the balance each event organiser deposits for covering the costs of
/// reserving balances, we define this trait to handle the metering of the balances that are taken
/// during the executing of the contract, and reverts the execution of it in case the balance is not
/// enough to cover for the costs.
///
/// This is to ensure that no event organiser would be subsidising another event's costs.
pub trait WithMeteredBalance:
    StaticEnv<EnvAccess = EnvAccess<'static, KreivoApiEnvironment>>
{
    /// Measures the changes in balance, and enforces the execution doesn't incur in
    /// [`LowBalance`][Error::LowBalance].
    fn with_metered_balance<R>(
        &self,
        event_id: &EventId,
        f: impl FnOnce() -> Result<R, Error>,
    ) -> Result<(R, Balance), Error> {
        let key_deposit_balance = (
            TICKETTO_ATTR,
            TickettoAttribute::Event(EventAttribute::DepositBalance),
        );

        let pre_balance = Self::env().balance();
        let transferred_value = Self::env().transferred_value();

        let event_balance: Balance = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
            &Self::env(),
            event_id,
            &key_deposit_balance,
        )
        .unwrap_or_default();

        let r = f()?;

        let post_balance = Self::env().balance();

        // No balance was spent, or even some balance was released. Also, some balance may have
        // been provided.
        let difference = if post_balance >= pre_balance {
            let difference = post_balance
                .checked_sub(pre_balance)
                .ok_or(Error::LowBalance)?;

            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                &Self::env(),
                event_id,
                &key_deposit_balance,
                &event_balance
                    .checked_add(transferred_value)
                    .ok_or(Error::Overflow)?
                    .checked_add(difference)
                    .ok_or(Error::Overflow)?,
            )?;

            difference
        } else {
            let difference = pre_balance
                .checked_sub(post_balance)
                .ok_or(Error::LowBalance)?;

            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                &Self::env(),
                event_id,
                &key_deposit_balance,
                &event_balance
                    .checked_add(transferred_value)
                    .ok_or(Error::Overflow)?
                    .checked_sub(difference)
                    .ok_or(Error::LowBalance)?,
            )?;

            difference
        };

        Ok((r, difference))
    }
}

pub trait WithAttributes: StaticEnv<EnvAccess = EnvAccess<'static, KreivoApiEnvironment>> {
    fn key(attribute: impl Into<TickettoAttribute>) -> impl Encode {
        let attr: TickettoAttribute = attribute.into();
        (TICKETTO_ATTR, attr)
    }

    fn get_attribute<V: Encode + Decode>(
        &self,
        event_id: &EventId,
        ticket_id: Option<&TicketId>,
        key: impl Into<TickettoAttribute>,
    ) -> Option<V> {
        if let Some(id) = ticket_id {
            <KreivoApi as KreivoAPI<_>>::Listings::item_attribute(
                &Self::env(),
                event_id,
                id,
                &Self::key(key),
            )
        } else {
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                &Self::env(),
                event_id,
                &Self::key(key),
            )
        }
    }

    fn set_attribute<V: Encode + Decode>(
        &self,
        event_id: &EventId,
        ticket_id: Option<&TicketId>,
        key: impl Into<TickettoAttribute>,
        value: &V,
    ) -> Result<(), Error> {
        if let Some(id) = ticket_id {
            <KreivoApi as KreivoAPI<_>>::Listings::item_set_attribute(
                &Self::env(),
                event_id,
                id,
                &Self::key(key),
                value,
            )
        } else {
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_set_attribute(
                &Self::env(),
                event_id,
                &Self::key(key),
                value,
            )
        }
        .map_err(|e| e.into())
    }

    fn clear_attribute(
        &self,
        event_id: &EventId,
        ticket_id: Option<&TicketId>,
        key: impl Into<TickettoAttribute>,
    ) -> Result<(), Error> {
        if let Some(id) = ticket_id {
            <KreivoApi as KreivoAPI<_>>::Listings::item_clear_attribute(
                &Self::env(),
                event_id,
                id,
                &Self::key(key),
            )
        } else {
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_clear_attribute::<_, ()>(
                &Self::env(),
                event_id,
                &Self::key(key),
            )
        }
        .map_err(|e| e.into())
    }
}

pub trait WithState: WithAttributes {
    fn with_state<R>(&self, event_id: &EventId, f: impl FnOnce(EventState) -> R) -> R {
        let state: EventState = self
            .get_attribute(event_id, None, EventAttribute::State)
            .unwrap_or_default();
        f(state)
    }

    fn ensure_state(
        &self,
        event_id: &EventId,
        f: impl FnOnce(EventState) -> bool,
    ) -> Result<(), Error> {
        self.with_state(event_id, |state| {
            f(state).then_some(()).ok_or(Error::InvalidState)
        })
    }
}

pub trait GetEventInfo: WithAttributes {
    fn get_event_info(&self, event_id: &EventId) -> Option<EventInfo> {
        Some(EventInfo {
            organiser: self.get_attribute(event_id, None, EventAttribute::Organiser)?,
            name: self.get_attribute(event_id, None, EventAttribute::Name)?,
            state: self
                .get_attribute(event_id, None, EventAttribute::State)
                .unwrap_or_default(),
            capacity: self.get_attribute(event_id, None, EventAttribute::Capacity)?,
            class: self.get_attribute(
                event_id,
                None,
                EventAttribute::TicketClass(DEFAULT_CLASS.into()),
            )?,
            dates: self.get_attribute(event_id, None, EventAttribute::Dates),
        })
    }
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !$cond {
            return Err($err.into()); // Into<E> lets callers use their own error type
        }
    };
}
