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
    use ink::codegen::Env;
    pub use kreivo_apis::{
        apis::{KreivoAPI, ListingsInventoriesAPI, ListingsItemsAPI},
        KreivoApi, KreivoApiEnvironment,
    };
    use ticketto_traits::*;
    pub use ticketto_types::*;

    /// Allows ticket holders to use their tickets.
    #[ink(storage)]
    #[derive(Default, PartialEq, Eq)]
    pub struct TickettoTickets {}

    /// Emits when a ticket is issued.
    #[ink(event)]
    pub struct TicketIssued {
        event: EventId,
        id: TicketId,
    }

    /// Emits when a ticket marks an attendance.
    #[ink(event)]
    pub struct TicketAttendance {
        event: EventId,
        id: TicketId,
        when: Timestamp,
    }

    /// Emits when a pending transfer is initiated.
    #[ink(event)]
    pub struct TicketPendingTransferInitiated {
        event: EventId,
        id: TicketId,
        beneficiary: AccountId,
    }

    impl WithMeteredBalance for TickettoTickets {}
    impl WithAttributes for TickettoTickets {}
    impl WithState for TickettoTickets {}
    impl GetEventInfo for TickettoTickets {}

    // Calls
    impl TickettoTickets {
        /// Default constructor.
        #[ink(constructor, selector = 0xFFFFFFFF)]
        pub fn initialize() -> Self {
            Self {}
        }

        /// Permissionlessly issues a new ticket. The new ticket takes the parameters given by the
        /// `EventInfo`. Tickets can only be issued on the [`Sales`][EventState::Sales] and
        /// [`Ongoing`][EventState::Ongoing] period, until maximum capacity is reached.
        ///
        /// Once the event finishes, or the maximum capacity is reached, it won't be longer possible
        /// to issue more tickets.
        #[ink(message, payable)]
        pub fn issue_ticket(&mut self, event_id: EventId) -> Result<Balance, Error> {
            let EventInfo {
                organiser,
                name,
                capacity,
                class:
                    TicketClass {
                        price,
                        attendance_policy,
                        maybe_restrictions,
                    },
                state,
                ..
            } = self.get_event_info(&event_id).ok_or(Error::EventNotFound)?;

            ensure!(
                matches!(state, EventState::Sales | EventState::Ongoing),
                Error::InvalidState
            );

            self.with_metered_balance(&event_id, || {
                let ticket_id = self.get_ticket_id(&event_id)?;
                ensure!(ticket_id < capacity, Error::MaxCapacity);

                // Issues a new ticket.
                <KreivoApi as KreivoAPI<_>>::Listings::publish(
                    &self.env(),
                    &event_id,
                    &ticket_id,
                    name,
                    Some(price),
                )?;

                self.set_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::AttendancePolicy,
                    &attendance_policy,
                )?;

                if let Some(restrictions) = maybe_restrictions {
                    if restrictions.cannot_resale {
                        <KreivoApi as KreivoAPI<_>>::Listings::item_disable_resell(
                            &self.env(),
                            &event_id,
                            &ticket_id,
                        )?;
                    }

                    if restrictions.cannot_transfer {
                        <KreivoApi as KreivoAPI<_>>::Listings::item_disable_transfer(
                            &self.env(),
                            &event_id,
                            &ticket_id,
                        )?;
                    }
                }

                // We transfer the ownership of the ticket to the event organiser.
                // This makes possible for the organiser to get the earnings of a ticket once
                // sold via the `Orders` pallet.
                <KreivoApi as KreivoAPI<_>>::Listings::item_creator_transfer(
                    &self.env(),
                    &event_id,
                    &ticket_id,
                    &organiser.into(),
                )?;

                self.env().emit_event(TicketIssued {
                    event: event_id,
                    id: ticket_id,
                });

                Ok(())
            })
            .map(|(_, b)| b)
        }

        /// Marks an attendance on a ticket. Only the ticket holder can mark it, and the success
        /// of the attendance
        #[ink(message, payable)]
        pub fn mark_attendance(
            &mut self,
            event_id: EventId,
            id: TicketId,
        ) -> Result<Balance, Error> {
            self.ensure_owner(&event_id, &id)?;
            self.ensure_state(&event_id, |state| state == EventState::Ongoing)?;

            self.with_metered_balance(&event_id, || {
                let now = &self.env().block_timestamp();
                let event_dates: EventDates = self
                    .get_attribute(&event_id, None, EventAttribute::Dates)
                    .unwrap_or_default();

                // Ensures we're within one of the events dates.
                event_dates
                    .iter()
                    .find(|(start, end)| start <= now && now < end)
                    .ok_or(Error::AttendanceOutOfDates)?;

                // Ensure we meet the attendance policy.
                let mut attendances: Attendances = self
                    .get_attribute(&event_id, Some(&id), TicketAttribute::Attendances)
                    .unwrap_or_default();
                let attendance_policy: AttendancePolicy = self
                    .get_attribute(&event_id, Some(&id), TicketAttribute::AttendancePolicy)
                    .ok_or(Error::TicketNotFound)?;

                if match &attendance_policy {
                    AttendancePolicy::Single => attendances.count == 1,
                    AttendancePolicy::Multiple { max, maybe_until } => {
                        matches!(maybe_until, Some(until) if now >= until)
                            || &attendances.count == max
                    }
                    AttendancePolicy::Unlimited { maybe_until } => {
                        matches!(maybe_until, Some(until) if now >= until)
                    }
                } {
                    Err(Error::MaxAttendances)?
                }

                attendances.count = attendances.count.saturating_add(1);
                attendances.last = *now;

                self.set_attribute(
                    &event_id,
                    Some(&id),
                    TicketAttribute::Attendances,
                    &attendances,
                )?;

                self.env().emit_event(TicketAttendance {
                    event: event_id,
                    id,
                    when: *now,
                });

                Ok(())
            })
            .map(|(_, b)| b)
        }

        /// Allows the ticket owner to initiate a pending transfer.
        #[ink(message, payable)]
        pub fn initiate_pending_transfer(
            &self,
            event_id: EventId,
            ticket_id: TicketId,
            beneficiary: AccountId,
        ) -> Result<Balance, Error> {
            self.ensure_owner(&event_id, &ticket_id)?;
            self.with_metered_balance(&event_id, || {
                self.set_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::PendingTransfer,
                    &beneficiary,
                )?;

                self.env().emit_event(TicketPendingTransferInitiated {
                    event: event_id,
                    id: ticket_id,
                    beneficiary,
                });

                Ok(())
            })
            .map(|(_, b)| b)
        }

        /// Allows the beneficiary of a pending ticket transfer to accept and receive the
        /// ticket.
        #[ink(message, payable)]
        pub fn accept_pending_transfer(
            &self,
            event_id: EventId,
            ticket_id: TicketId,
        ) -> Result<Balance, Error> {
            let who = self.env().caller();
            let TicketInfo {
                pending_transfer, ..
            } = self.get(event_id, ticket_id).ok_or(Error::TicketNotFound)?;
            let beneficiary = pending_transfer.ok_or(Error::NoPendingTransfer)?;

            ensure!(who == beneficiary.into(), Error::NoPermission);

            self.with_metered_balance(&event_id, || {
                self.clear_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::PendingTransfer,
                )?;

                <KreivoApi as KreivoAPI<_>>::Listings::item_transfer(
                    &self.env(),
                    &event_id,
                    &ticket_id,
                    &beneficiary.into(),
                )
                .map_err(|e| e.into())
            })
            .map(|(_, b)| b)
        }

        /// Allows the sender or the beneficiary of a pending ticket transfer to rescind it.
        #[ink(message, payable)]
        pub fn cancel_pending_transfer(
            &self,
            event_id: EventId,
            ticket_id: TicketId,
        ) -> Result<Balance, Error> {
            let who = self.env().caller();
            let TicketInfo {
                owner,
                pending_transfer,
                ..
            } = self.get(event_id, ticket_id).ok_or(Error::TicketNotFound)?;
            let beneficiary = pending_transfer.ok_or(Error::NoPendingTransfer)?;

            ensure!(
                who == owner.into() || who == beneficiary.into(),
                Error::NoPermission
            );

            self.with_metered_balance(&event_id, || {
                self.clear_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::PendingTransfer,
                )
            })
            .map(|(_, b)| b)
        }
    }

    // Storage
    impl TickettoTickets {
        #[ink(message)]
        pub fn get(&self, event_id: EventId, ticket_id: TicketId) -> Option<TicketInfo> {
            let item =
                <KreivoApi as KreivoAPI<_>>::Listings::item(&self.env(), &event_id, &ticket_id)?;
            let event_name = self.get_attribute(&event_id, None, EventAttribute::Name)?;

            Some(TicketInfo {
                name: event_name,
                owner: item.owner.0,
                attendance_policy: self.get_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::AttendancePolicy,
                )?,
                attendances: self
                    .get_attribute(&event_id, Some(&ticket_id), TicketAttribute::Attendances)
                    .unwrap_or_default(),
                price: item.price,
                restrictions: TicketRestrictions {
                    cannot_resale: !<KreivoApi as KreivoAPI<_>>::Listings::item_can_resell(
                        &self.env(),
                        &event_id,
                        &ticket_id,
                    ),
                    cannot_transfer: !<KreivoApi as KreivoAPI<_>>::Listings::item_transferable(
                        &self.env(),
                        &event_id,
                        &ticket_id,
                    ),
                },
                pending_transfer: self.get_attribute(
                    &event_id,
                    Some(&ticket_id),
                    TicketAttribute::PendingTransfer,
                ),
            })
        }

        #[ink(message)]
        pub fn attendances(&self, event_id: EventId, ticket_id: TicketId) -> Attendances {
            self.get_attribute(&event_id, Some(&ticket_id), TicketAttribute::Attendances)
                .unwrap_or_default()
        }

        #[ink(message)]
        pub fn pending_transfer_for(
            &self,
            event_id: EventId,
            ticket_id: TicketId,
        ) -> Option<AccountId> {
            self.get_attribute(
                &event_id,
                Some(&ticket_id),
                TicketAttribute::PendingTransfer,
            )
        }
    }

    impl TickettoTickets {
        fn ensure_owner(&self, event_id: &EventId, id: &TicketId) -> Result<(), Error> {
            let item = <KreivoApi as KreivoAPI<_>>::Listings::item(&self.env(), event_id, id)
                .ok_or(Error::TicketNotFound)?;
            // Ensure the caller is the event organiser.
            (item.owner == self.env().caller())
                .then_some(())
                .ok_or(Error::NoPermission)
        }

        fn get_ticket_id(&self, event_id: &EventId) -> Result<TicketId, Error> {
            let next_ticket_id: TicketId = self
                .get_attribute(event_id, None, EventAttribute::NextTicketId)
                .unwrap_or_default();

            self.set_attribute(
                event_id,
                None,
                EventAttribute::NextTicketId,
                &next_ticket_id.checked_add(1).ok_or(Error::Overflow)?,
            )?;

            Ok(next_ticket_id)
        }
    }
}
