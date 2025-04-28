// Copyright 2025 The Kippu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License")
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

#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use ticketto_tickets::*;

#[cfg(test)]
mod tests;

#[ink::contract(env = KreivoApiEnvironment)]
mod ticketto_tickets {
    use codec::{Decode, Encode};
    use ink::{codegen::Env, prelude::vec::Vec};
    pub use kreivo_apis::{
        apis::{KreivoAPI, ListingsInventoriesAPI, ListingsItemsAPI},
        KreivoApi, KreivoApiEnvironment,
    };
    use ticketto_traits::WithMeteredBalance;
    pub use ticketto_types::{
        event_attributes, ticket_attributes, AttendancePolicy, Error, EventCapacity, EventId,
        EventInfo, EventName, ItemPrice, TicketId, TicketRestrictions,
    };
    use ticketto_types::{EventDates, EventState};

    /// Allows ticket holders to use their tickets.
    #[ink(storage)]
    #[derive(Default, PartialEq, Eq)]
    pub struct TickettoTickets {}

    /// Emits when a ticket marks an attendance.
    #[ink(event)]
    pub struct TicketAttendance {
        event: EventId,
        id: TicketId,
        when: Timestamp,
    }

    impl TickettoTickets {
        /// Default constructor.
        #[ink(constructor, selector = 0x00)]
        pub fn initialize() -> Self {
            Self {}
        }

        /// Marks an attendance on a ticket. Only the ticket holder can mark it, and the success
        /// of the attendance
        #[ink(message, payable, selector = 0x01)]
        pub fn mark_attendance(
            &mut self,
            event_id: EventId,
            id: TicketId,
        ) -> Result<Balance, Error> {
            self.ensure_owner(&event_id, &id)?;
            self.with_state(&event_id, |state| state == EventState::Ongoing)?;

            let now = &self.env().block_timestamp();
            let event_dates: EventDates =
                <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                    &self.env(),
                    &event_id,
                    &event_attributes::DATES,
                )
                .unwrap_or_default();

            // Ensures we're within one of the events dates.
            event_dates
                .iter()
                .find(|(start, end)| start <= now && now < end)
                .ok_or(Error::AttendanceOutOfDates)?;

            // Ensure we meet the attendance policy.
            let mut attendances: Vec<Timestamp> = self
                .get_attribute(&event_id, &id, &ticket_attributes::ATTENDANCES)
                .unwrap_or_default();
            let attendance_policy: AttendancePolicy = self
                .get_attribute(&event_id, &id, &ticket_attributes::ATTENDANCE_POLICY)
                .ok_or(Error::TicketNotFound)?;

            if match attendance_policy {
                AttendancePolicy::Single => attendances.len() == 1,
                AttendancePolicy::Multiple { max, maybe_until } => {
                    matches!(maybe_until, Some(until) if now >= &until)
                        || u32::try_from(attendances.len()).map_err(|_| Error::MaxAttendances)?
                            == max
                }
                AttendancePolicy::Unlimited { maybe_until } => {
                    matches!(maybe_until, Some(until) if now >= &until)
                }
            } {
                Err(Error::MaxAttendances)?
            }

            attendances.push(*now);

            self.set_attribute(
                &event_id,
                &id,
                &ticket_attributes::ATTENDANCES,
                &attendances,
            )
            .map_err(|_| Error::MaxAttendances)
            .and_then(|b| {
                self.env().emit_event(TicketAttendance {
                    event: event_id,
                    id,
                    when: now,
                });

                Ok(b)
            })
        }
    }

    impl WithMeteredBalance for TickettoTickets {}

    impl TickettoTickets {
        fn ensure_owner(&self, event_id: &EventId, id: &TicketId) -> Result<(), Error> {
            let item = <KreivoApi as KreivoAPI<_>>::Listings::item(&self.env(), event_id, id)
                .ok_or(Error::TicketNotFound)?;
            // Ensure the caller is the event organiser.
            (item.owner == self.env().caller())
                .then_some(())
                .ok_or(Error::NoPermission)
        }

        fn with_state(
            &self,
            event_id: &EventId,
            f: impl FnOnce(EventState) -> bool,
        ) -> Result<(), Error> {
            let state: EventState = <KreivoApi as KreivoAPI<_>>::Listings::inventory_attribute(
                &self.env(),
                event_id,
                &event_attributes::STATE,
            )
            .unwrap_or_default();

            f(state).then_some(()).ok_or(Error::InvalidState)
        }

        fn get_attribute<K: Encode, V: Encode + Decode>(
            &self,
            event_id: &EventId,
            id: &TicketId,
            attribute: &K,
        ) -> Option<V> {
            <KreivoApi as KreivoAPI<_>>::Listings::item_attribute(
                &self.env(),
                event_id,
                id,
                attribute,
            )
        }

        fn set_attribute<K: Encode, V: Encode>(
            &self,
            event_id: &EventId,
            id: &TicketId,
            attribute: &K,
            value: &V,
        ) -> Result<Balance, Error> {
            Self::with_metered_balance(&mut self.env(), event_id, || {
                <KreivoApi as KreivoAPI<_>>::Listings::item_set_attribute(
                    &self.env(),
                    event_id,
                    id,
                    attribute,
                    value,
                )?;
                Ok(())
            })
            .map(|(_, r)| r)
        }
    }
}
