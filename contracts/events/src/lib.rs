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

pub use ticketto_events::*;

#[cfg(test)]
mod tests;

#[ink::contract(env = KreivoApiEnvironment)]
mod ticketto_events {
    use ink::codegen::Env;
    use scale_info::prelude::vec::Vec;

    pub use kreivo_apis::{
        apis::{KreivoAPI, ListingsInventoriesAPI, ListingsItemsAPI},
        KreivoApi, KreivoApiEnvironment,
    };
    use ticketto_traits::*;
    pub use ticketto_types::*;

    /// Allows organisers to manage their events.
    ///
    /// This implementation of `Ticketto` heavily rely upon the Kreivo APIs. This means we'll use
    /// the Listings Inventory APIs (which under the hood are a wrapping for namespaced nonfungibles
    /// collections).
    ///
    /// Hence, this contract only stores the counter for the next event, since all the remaining
    /// properties will be stored as attributes to the inventory, for which the deposit balance
    /// would be provided by the account that instantiates the contract.
    #[ink(storage)]
    #[derive(Default, PartialEq, Eq)]
    pub struct TickettoEvents {
        /// A counter that indicates the next [`EventId`]
        pub(crate) next_event_id: EventId,
    }

    /// Emits when an event is registered.
    #[ink(event)]
    pub struct EventRegistered {
        id: EventId,
        organiser: AccountId,
    }

    impl WithMeteredBalance for TickettoEvents {}
    impl WithAttributes for TickettoEvents {}
    impl WithState for TickettoEvents {}
    impl GetEventInfo for TickettoEvents {}

    // Calls
    impl TickettoEvents {
        /// Initialize the contract.
        #[ink(constructor, selector = 0xFFFFFFFF)]
        pub fn new() -> Self {
            Default::default()
        }

        /// A permissionless method, that allows anyone to cover the deposit account of the event.
        /// Fails if the given event does not exist.
        ///
        /// It is expected to have a zero difference, because it's a no-op (in storage terms).
        #[ink(message, payable, selector = 0xFFFFFFFE)]
        pub fn deposit(&mut self, event_id: EventId) -> Result<(), Error> {
            <KreivoApi as KreivoAPI<_>>::Listings::inventory_exists(&self.env(), &event_id)
                .then_some(())
                .ok_or(Error::EventNotFound)
        }

        /// Creates a new inventory for storing the event (and its details).
        ///
        /// The caller must assume the storage cost to create a new event, otherwise, a
        /// [`LowBalance`][Error::LowBalance] error will be thrown.
        #[ink(message, payable)]
        pub fn create_event(
            &mut self,
            name: EventName,
            capacity: EventCapacity,
            ticket_class: TicketClass,
            maybe_dates: Option<EventDates>,
            maybe_metadata: Option<Vec<u8>>,
        ) -> Result<(EventId, Balance), Error> {
            let state = &EventState::Created;
            let caller = self.env().caller();
            let event_id = self.get_event_id().ok_or(Error::Overflow)?;

            self.with_metered_balance(&event_id, || {
                // Create a new event.
                <KreivoApi as KreivoAPI<_>>::Listings::create(&self.env(), &event_id)?;

                // Sets the event organiser.
                self.set_attribute(&event_id, None, EventAttribute::Organiser, &caller)?;

                // Adds attributes to the event.
                self.set_name(&event_id, state, &name)?;
                self.set_capacity(&event_id, state, &capacity)?;
                self.set_ticket_class(&event_id, state, &ticket_class)?;

                // If given, sets the dates to the event.
                if let Some(dates) = &maybe_dates {
                    self.set_event_dates(&event_id, state, dates)?;
                }

                // If given, sets a reference to the metadata of the event.
                if let Some(metadata) = &maybe_metadata {
                    self.set_event_metadata(&event_id, state, metadata)?;
                }

                self.env().emit_event(EventRegistered {
                    id: event_id,
                    organiser: self.env().caller(),
                });

                Ok(event_id)
            })
        }

        #[ink(message, payable)]
        pub fn update(
            &self,
            event_id: EventId,
            maybe_name: Option<EventName>,
            maybe_capacity: Option<EventCapacity>,
            maybe_ticket_class: Option<TicketClass>,
            maybe_dates: Option<EventDates>,
            maybe_metadata: Option<Vec<u8>>,
        ) -> Result<Balance, Error> {
            self.ensure_organiser(&event_id)?;

            self.with_metered_balance(&event_id, || {
                self.with_state(&event_id, |ref state| {
                    if let Some(name) = maybe_name {
                        self.set_name(&event_id, state, &name)?;
                    }

                    if let Some(capacity) = maybe_capacity {
                        self.set_capacity(&event_id, state, &capacity)?;
                    }

                    if let Some(ticket_class) = maybe_ticket_class {
                        self.set_ticket_class(&event_id, state, &ticket_class)?;
                    }

                    if let Some(dates) = &maybe_dates {
                        self.set_event_dates(&event_id, state, dates)?;
                    }

                    if let Some(metadata) = &maybe_metadata {
                        self.set_event_metadata(&event_id, state, metadata)?;
                    }

                    Ok(())
                })
            })
            .map(|(_, b)| b)
        }

        #[ink(message, payable)]
        pub fn bump_state(&mut self, event_id: EventId) -> Result<Balance, Error> {
            self.with_metered_balance(&event_id, || {
                self.with_state(&event_id, |ref state| {
                    let dates: EventDates = self
                        .get_attribute(&event_id, None, EventAttribute::Dates)
                        .ok_or(Error::DatesNotSet)?;

                    let next_state = match state {
                        EventState::Created => {
                            // Only the organiser can bump to sales, once some dates have been set.
                            self.ensure_organiser(&event_id)?;
                            EventState::Sales
                        }
                        EventState::Sales => {
                            // Anyone can bump to ongoing, as long as the first date has been reached.
                            let (starts, _) = dates.first().ok_or(Error::DatesNotSet)?;
                            (starts <= &self.env().block_timestamp())
                                .then_some(())
                                .ok_or(Error::InvalidState)?;
                            EventState::Ongoing
                        }
                        // Anyone can bump to finished, as long as the last date has passed.
                        EventState::Ongoing => {
                            let (_, ends) = dates.last().ok_or(Error::DatesNotSet)?;
                            (ends <= &self.env().block_timestamp())
                                .then_some(())
                                .ok_or(Error::InvalidState)?;
                            EventState::Finished
                        }
                        // This is the last state, so it's final. Errors to prevent redundant
                        // overwriting and resources exhaustion.
                        EventState::Finished => Err(Error::InvalidState)?,
                    };

                    self.set_attribute(&event_id, None, EventAttribute::State, &next_state)?;
                    Ok(())
                })
            })
            .map(|(_, b)| b)
        }
    }

    // Storage
    impl TickettoEvents {
        #[ink(message)]
        pub fn get(&self, event_id: EventId) -> Option<EventInfo> {
            self.get_event_info(&event_id)
        }
    }

    impl TickettoEvents {
        fn get_event_id(&mut self) -> Option<EventId> {
            let event_id = self.next_event_id;
            self.next_event_id = self.next_event_id.checked_add(1)?;
            Some(event_id)
        }

        fn ensure_organiser(&self, event_id: &EventId) -> Result<(), Error> {
            let event_organiser: ink::primitives::AccountId = self
                .get_attribute(event_id, None, EventAttribute::Organiser)
                .ok_or(Error::EventNotFound)?;

            // Ensure the caller is the event organiser.
            (event_organiser == self.env().caller())
                .then_some(())
                .ok_or(Error::NoPermission)
        }

        /// Edits the name of the event.
        ///
        /// The method fails if the event tickets are already on sale.
        fn set_name(
            &self,
            event_id: &EventId,
            state: &EventState,
            name: &EventName,
        ) -> Result<(), Error> {
            ensure!(*state == EventState::Created, Error::InvalidState);
            self.set_attribute(event_id, None, EventAttribute::Name, name)
        }

        /// Edits the capacity of the event.
        ///
        /// The method fails if the event tickets are already on sale.
        fn set_capacity(
            &self,
            event_id: &EventId,
            state: &EventState,
            capacity: &EventCapacity,
        ) -> Result<(), Error> {
            ensure!(*state == EventState::Created, Error::InvalidState);
            self.set_attribute(event_id, None, EventAttribute::Capacity, capacity)
        }

        /// Edits the class of the ticket.
        ///
        /// The method fails if the event tickets are already on sale.
        fn set_ticket_class(
            &self,
            event_id: &EventId,
            state: &EventState,
            ticket_class: &TicketClass,
        ) -> Result<(), Error> {
            ensure!(*state == EventState::Created, Error::InvalidState);
            self.set_attribute(
                event_id,
                None,
                EventAttribute::TicketClass(DEFAULT_CLASS.into()),
                ticket_class,
            )
        }

        /// Sets the dates of the event.
        ///
        /// The method fails if the event is ongoing or finished.
        fn set_event_dates(
            &self,
            event_id: &EventId,
            state: &EventState,
            dates: &EventDates,
        ) -> Result<(), Error> {
            ensure!(
                matches!(state, EventState::Created | EventState::Sales),
                Error::InvalidState
            );
            self.set_attribute(event_id, None, EventAttribute::Dates, dates)
        }

        /// Sets the dates of the event.
        ///
        /// The method fails if the event is ongoing or finished.
        fn set_event_metadata(
            &self,
            event_id: &EventId,
            state: &EventState,
            metadata: &[u8],
        ) -> Result<(), Error> {
            ensure!(
                matches!(state, EventState::Created | EventState::Sales),
                Error::InvalidState
            );
            <KreivoApi as KreivoAPI<_>>::Listings::set_inventory_metadata(
                &self.env(),
                event_id,
                metadata,
            )
            .map_err(|e| e.into())
        }
    }
}
